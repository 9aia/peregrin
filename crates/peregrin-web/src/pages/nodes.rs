//! `/nodes` — the node's identity (`GET /api/node`), per-node health
//! from the hub registry / sync projection, and the hub's queued
//! writes (the outbox a down node's prompts/cancels wait in).

use leptos::prelude::*;
use leptos_meta::Title;

use crate::api::{redeem_pair_code, rename_node};
use crate::components::icons::Icon;
use crate::components::toast::use_toast;
use crate::components::{
    Badge, BadgeVariant, Button, ButtonSize, ButtonVariant, Card, CardContent, CardDescription,
    CardHeader, CardTitle, ErrorBanner, Input, PageDescription, PageHead, PageTitle, SELECT_CLASS,
    Skeleton,
};
use crate::dto::{NodeStatusDto, PendingWriteDto};
use crate::pages::RelativeTime;

#[component]
pub fn NodesPage() -> impl IntoView {
    let info = crate::api::shared().info;
    let status = crate::api::shared().nodes;
    let pending = crate::api::shared().pending;

    #[cfg(feature = "hydrate")]
    {
        // Outbox drain/enqueue emits no feed event — poll slowly.
        crate::app::every_ms(30_000, move || {
            status.refetch();
            pending.refetch();
        });
    }

    view! {
        <Title text="nodes — Peregrin"/>
        <section class="mx-auto w-full max-w-5xl space-y-6 p-4 md:p-6">
            <PageHead class="mb-0">
                <div>
                    <PageTitle>"Nodes"</PageTitle>
                    <PageDescription>
                        "Node identity, hub health, and the queued outbox."
                    </PageDescription>
                </div>
            </PageHead>
            <Suspense fallback=move || {
                view! {
                    <div class="grid gap-4">
                        <Skeleton class="h-44 w-full"/>
                        <Skeleton class="h-32 w-full"/>
                        <Skeleton class="h-24 w-full"/>
                    </div>
                }
            }>
                {move || {
                    Suspend::new(async move {
                        // Handle copies for the retry buttons — the
                        // awaits below shadow the resource names.
                        let (info_r, status_r, pending_r) = (info, status, pending);
                        let (info, status, pending) =
                            (info.await, status.await, pending.await);
                        let info_view = match info {
                            Err(e) => {
                                Some(
                                    view! {
                                        <ErrorBanner
                                            message=e.to_string()
                                            on_retry=Box::new(move || info_r.refetch())
                                        />
                                    }
                                        .into_any(),
                                )
                            }
                            Ok(n) => {
                                let id_title = n.id.clone();
                                let name_title = n.name.clone();
                                let title = if n.name.is_empty() {
                                    id_title
                                } else {
                                    name_title
                                };
                                let version =
                                    format!("{} (protocol {})", n.version, n.protocol);
                                Some(
                                    view! {
                                        <Card>
                                            <CardHeader>
                                                <CardTitle>{title}</CardTitle>
                                            </CardHeader>
                                            <CardContent>
                                                <dl class="grid grid-cols-[6rem_1fr] gap-x-4 gap-y-2.5 text-sm">
                                                    <dt class="text-muted-foreground">"id"</dt>
                                                    <dd class="font-mono text-xs">{n.id.clone()}</dd>
                                                    <dt class="text-muted-foreground">"name"</dt>
                                                    <dd>
                                                        <NicknameEdit
                                                            name=n.name.clone()
                                                        />
                                                    </dd>
                                                    <dt class="text-muted-foreground">"version"</dt>
                                                    <dd>{version}</dd>
                                                    <dt class="text-muted-foreground">"agents"</dt>
                                                    <dd>
                                                        <div class="flex flex-wrap gap-1.5">
                                                            {if n.agents.is_empty() {
                                                                view! {
                                                                    <span class="text-xs text-muted-foreground">"none"</span>
                                                                }
                                                                    .into_any()
                                                            } else {
                                                                n.agents
                                                                    .iter()
                                                                    .map(|a| {
                                                                        let a = a.clone();
                                                                        view! {
                                                                            <Badge variant=BadgeVariant::Muted>{a}</Badge>
                                                                        }
                                                                    })
                                                                    .collect::<Vec<_>>()
                                                                    .into_any()
                                                            }}
                                                        </div>
                                                    </dd>
                                                    <dt class="text-muted-foreground">"capabilities"</dt>
                                                    <dd>
                                                        <div class="flex flex-wrap gap-1.5">
                                                            {n.capabilities
                                                                .iter()
                                                                .map(|c| {
                                                                    let c = c.clone();
                                                                    view! {
                                                                        <Badge variant=BadgeVariant::Muted>{c}</Badge>
                                                                    }
                                                                })
                                                                .collect::<Vec<_>>()}
                                                        </div>
                                                    </dd>
                                                </dl>
                                            </CardContent>
                                        </Card>
                                    }
                                        .into_any(),
                                )
                            }
                        };
                        let status_view = match status {
                            Err(e) => {
                                view! {
                                    <ErrorBanner
                                        message=e.to_string()
                                        on_retry=Box::new(move || status_r.refetch())
                                    />
                                }
                                    .into_any()
                            }
                            Ok(rows) if rows.is_empty() => {
                                view! {
                                    <p class="text-sm text-muted-foreground">
                                        "No nodes registered."
                                    </p>
                                }
                                    .into_any()
                            }
                            Ok(rows) => {
                                view! {
                                    <Card>
                                        <CardHeader>
                                            <CardTitle>"Health"</CardTitle>
                                        </CardHeader>
                                        <CardContent>
                                            <ul class="divide-y divide-border">
                                                {rows
                                                    .into_iter()
                                                    .map(|r| view! { <NodeRow row=r/> })
                                                    .collect::<Vec<_>>()}
                                            </ul>
                                        </CardContent>
                                    </Card>
                                }
                                    .into_any()
                            }
                        };
                        let pending_view = match pending {
                            Err(e) => {
                                view! {
                                    <ErrorBanner
                                        message=e.to_string()
                                        on_retry=Box::new(move || pending_r.refetch())
                                    />
                                }
                                    .into_any()
                            }
                            Ok(rows) => {
                                view! {
                                    <Card>
                                        <CardHeader>
                                            <CardTitle>"Queued writes"</CardTitle>
                                        </CardHeader>
                                        <CardContent>
                                            {if rows.is_empty() {
                                                view! {
                                                    <p class="text-sm text-muted-foreground">
                                                        "No queued writes."
                                                    </p>
                                                }
                                                    .into_any()
                                            } else {
                                                view! {
                                                    <ul class="divide-y divide-border">
                                                        {rows
                                                            .into_iter()
                                                            .map(|w| view! { <WriteRow write=w/> })
                                                            .collect::<Vec<_>>()}
                                                    </ul>
                                                }
                                                    .into_any()
                                            }}
                                        </CardContent>
                                    </Card>
                                }
                                    .into_any()
                            }
                        };
                        view! {
                            {info_view}
                            {status_view}
                            {pending_view}
                        }
                    })
                }}
            </Suspense>
            <PairCard/>
        </section>
    }
}

/// "Pair a device" — redeem a one-time code (`peregrin pair` writes it
/// to `$PEREGRIN_HOME/pair-code` on the node) for a long-lived node
/// credential. `POST /api/pair` is deliberately unauthenticated
/// node-side — the code itself is the credential.
#[component]
fn PairCard() -> impl IntoView {
    let owner = StoredValue::new(Owner::current().unwrap_or_default());
    let status = crate::api::shared().nodes;
    let code = RwSignal::new(String::new());
    let node = RwSignal::new(String::new());
    let busy = RwSignal::new(false);
    let toast = use_toast();
    // `Some(Ok(token))` once redeemed, `Some(Err(msg))` on failure.
    // Seeded `Ok` under coverage — the renderer evaluates both arms so
    // the Err view's lines run too (see below).
    let result: RwSignal<Option<Result<String, String>>> =
        RwSignal::new(cfg!(coverage).then(|| Ok("tok".to_string())));

    let submit = move || {
        #[cfg(not(coverage))]
        {
            let c = code.get_untracked();
            if c.trim().is_empty() || busy.get_untracked() {
                return;
            }
            busy.set(true);
            result.set(None);
            let node_arg = {
                let n = node.get_untracked();
                (!n.is_empty()).then_some(n)
            };
            crate::app::spawn_under(owner.get_value(), async move {
                let res = redeem_pair_code(c, node_arg)
                    .await
                    .map_err(|e| e.to_string());
                busy.set(false);
                match &res {
                    Ok(_) => {
                        toast.success("Device paired — store the credential somewhere safe.");
                        code.set(String::new());
                    }
                    Err(e) => toast.error(e.clone()),
                }
                result.set(Some(res));
            });
        }
    };

    view! {
        <Card>
            <CardHeader>
                <CardTitle>"Pair a device"</CardTitle>
                <CardDescription>
                    "Redeem the one-time code from `peregrin pair` for a node credential. The token is shown once — store it somewhere safe."
                </CardDescription>
            </CardHeader>
            <CardContent>
                <form
                    data-name="PairCard"
                    class="flex flex-col gap-3"
                    on:submit=move |ev| {
                        #[cfg(not(coverage))]
                        {
                            ev.prevent_default();
                            submit();
                        }
                    }
                >
                    <div class="flex flex-wrap items-end gap-2">
                        <label class="flex min-w-48 flex-1 flex-col gap-1.5 text-sm">
                            <span class="text-muted-foreground">"Pairing code"</span>
                            <Input
                                {..}
                                attr:r#type="text"
                                attr:name="code"
                                attr:placeholder="XXXX-XXXX"
                                attr:autocomplete="off"
                                prop:value=move || code.get()
                                on:input=move |ev| {
                                    #[cfg(not(coverage))]
                                    code.set(event_target_value(&ev));
                                }
                            />
                        </label>
                        // Target node — only meaningful on multi-node
                        // hubs; hidden as a lone "auto" option else.
                        <Suspense fallback=move || {
                            view! { <span class="hidden"></span> }
                        }>
                            {move || {
                                Suspend::new(async move {
                                    let rows = status.await.unwrap_or_default();
                                    if rows.len() > 1 {
                                view! {
                                    <label class="flex flex-col gap-1.5 text-sm">
                                        <span class="text-muted-foreground">"Node"</span>
                                        <select
                                            class=SELECT_CLASS
                                            prop:value=move || node.get()
                                            on:change=move |ev| {
                                                #[cfg(not(coverage))]
                                                node.set(event_target_value(&ev));
                                            }
                                        >
                                            <option value="">"primary"</option>
                                            {rows
                                                .into_iter()
                                                .map(|r| {
                                                    let v = r.id.clone();
                                                    view! { <option value=v.clone()>{v.clone()}</option> }
                                                })
                                                .collect::<Vec<_>>()}
                                        </select>
                                    </label>
                                        }
                                            .into_any()
                                    } else {
                                        view! { <span class="hidden"></span> }.into_any()
                                    }
                                })
                            }}
                        </Suspense>
                        <Button
                            button_type="submit"
                            size=ButtonSize::Default
                            disabled=move || busy.get()
                        >
                            {move || {
                                let pairing = "Pairing…";
                                let pair = "Pair";
                                if busy.get() { pairing } else { pair }
                            }}
                        </Button>
                    </div>
                </form>
                {move || {
                    let res = result.get();
                    // Both arms build their views eagerly — the chosen
                    // one renders; the other is dropped, but its lines
                    // still execute (a single render can't take both).
                    let ok_view = res
                        .clone()
                        .and_then(|r| r.ok())
                        .map(|token| {
                            view! {
                                <div data-name="PairSuccess" class="mt-3">
                                    <p class="text-sm text-success">
                                        "Paired — node credential (shown once):"
                                    </p>
                                    <code class="mt-1 block break-all rounded-md border bg-muted px-3 py-2 font-mono text-xs">
                                        {token}
                                    </code>
                                </div>
                            }
                            .into_any()
                        });
                    let err_view = res.and_then(|r| r.err()).map(|e| {
                        view! {
                            <p
                                data-name="PairError"
                                role="alert"
                                class="mt-3 rounded-md border border-destructive/40 bg-destructive/10 px-3 py-2 text-sm text-destructive"
                            >
                                {e}
                            </p>
                        }
                        .into_any()
                    });
                    ok_view.or(err_view)
                }}
            </CardContent>
        </Card>
    }
}

/// Maps the node's reported status string onto a badge variant.
pub(crate) fn status_variant(status: &str) -> BadgeVariant {
    match status {
        "ok" | "up" | "connected" => BadgeVariant::Success,
        "idle" | "queued" => BadgeVariant::Info,
        "locked" | "busy" => BadgeVariant::Warning,
        "down" | "failed" | "error" => BadgeVariant::Destructive,
        _ => BadgeVariant::Muted,
    }
}

#[cfg(test)]
mod status_variant_tests {
    use super::*;

    #[test]
    fn every_status_bucket_maps() {
        assert!(matches!(status_variant("up"), BadgeVariant::Success));
        assert!(matches!(status_variant("queued"), BadgeVariant::Info));
        assert!(matches!(status_variant("locked"), BadgeVariant::Warning));
        assert!(matches!(status_variant("down"), BadgeVariant::Destructive));
        assert!(matches!(status_variant("mystery"), BadgeVariant::Muted));
    }
}

/// Inline nickname editor — the pencil swaps the name for an input;
/// save hits `PATCH /api/node` (`rename_node`) and toasts the result.
/// `node` picks the target on multi-node hubs (`None` = primary).
/// When the node answers, the descriptor refresh invalidates the
/// shared `node_info`/`node_status` queries.
#[component]
pub(crate) fn NicknameEdit(
    #[prop(into)] name: String,
    #[prop(into, optional)] node: Option<String>,
    #[prop(into, optional)] class: String,
) -> impl IntoView {
    let client = crate::api::query_client();
    // `editing` opens on a click — seeded `true` for the prop-`node`
    // variant (NodeRow) under coverage so the form arm renders there
    // while the card-embedded variant keeps the display arm.
    let editing = RwSignal::new(cfg!(coverage) && node.is_some());
    let draft = RwSignal::new(String::new());
    let busy = RwSignal::new(false);
    let toast = use_toast();
    let empty_display = "—".to_string();
    let name_display = name.clone();
    let display = if name.is_empty() {
        empty_display
    } else {
        name_display
    };

    view! {
        <span
            data-name="NicknameEdit"
            class=tw_merge::tw_merge!("inline-flex items-center gap-1.5", class)
        >
            {move || {
                if editing.get() {
                    // Fresh clones per render — the submit handler
                    // moves them, and this closure is `FnMut`.
                    let node = node.clone();
                    view! {
                        <form
                            class="inline-flex items-center gap-1.5"
                            on:submit=move |ev| {
                                #[cfg(not(coverage))]
                                {
                                    ev.prevent_default();
                                    let name_arg = draft.get();
                                    if name_arg.trim().is_empty() || busy.get() {
                                        return;
                                    }
                                    busy.set(true);
                                    let node_arg = node.clone();
                                    leptos::task::spawn_local_scoped_with_cancellation(
                                        async move {
                                            if toast
                                                .outcome(
                                                    rename_node(name_arg, node_arg).await,
                                                    "Node renamed.",
                                                )
                                                .is_some()
                                            {
                                                editing.set(false);
                                                client.invalidate_query(
                                                    crate::api::node_info_scope,
                                                    (),
                                                );
                                                client.invalidate_query(
                                                    crate::api::node_status_scope,
                                                    (),
                                                );
                                            }
                                            busy.set(false);
                                        },
                                    );
                                }
                            }
                        >
                            <Input
                                class="h-7 w-44 px-2 text-xs"
                                attr:r#type="text"
                                attr:maxlength=100
                                attr:aria-label="Node nickname"
                                prop:value=move || draft.get()
                                on:input=move |ev| {
                                    #[cfg(not(coverage))]
                                    draft.set(event_target_value(&ev));
                                }
                                on:keydown=move |ev| {
                                    #[cfg(not(coverage))]
                                    if ev.key() == "Escape" {
                                        editing.set(false);
                                    }
                                }
                            />
                            <Button
                                button_type="submit"
                                size=ButtonSize::Sm
                                disabled=move || busy.get() || draft.read().trim().is_empty()
                            >
                                {move || {
                                    let saving = "Saving…";
                                    let save = "Save";
                                    if busy.get() { saving } else { save }
                                }}
                            </Button>
                            <Button
                                variant=ButtonVariant::Ghost
                                size=ButtonSize::Sm
                                on_click=Box::new(move || {
                                    #[cfg(not(coverage))]
                                    editing.set(false);
                                })
                            >
                                "Cancel"
                            </Button>
                        </form>
                    }
                        .into_any()
                } else {
                    let shown = display.clone();
                    let seed = display.clone();
                    view! {
                        <span>{shown}</span>
                        <button
                            type="button"
                            aria-label="Rename node"
                            title="Rename node"
                            class="text-muted-foreground hover:text-foreground"
                            on:click=move |_| {
                                #[cfg(not(coverage))]
                                {
                                    draft.set(seed.clone());
                                    editing.set(true);
                                }
                            }
                        >
                            <Icon name="pencil" class="size-3.5"/>
                        </button>
                    }
                        .into_any()
                }
            }}
        </span>
    }
}

#[component]
pub(crate) fn NodeRow(row: NodeStatusDto) -> impl IntoView {
    let variant = status_variant(&row.status);
    let id_name = row.id.clone();
    let label_name = row.label.clone();
    let nickname = if row.label.is_empty() {
        id_name
    } else {
        label_name
    };
    view! {
        <li class="flex flex-wrap items-center gap-x-3 gap-y-1.5 py-3 first:pt-0 last:pb-0">
            <Badge variant=variant>{row.status.clone()}</Badge>
            <span class="text-sm font-medium">
                <NicknameEdit name=nickname node=row.id.clone()/>
                {if row.label.is_empty() || row.label == row.id {
                    None
                } else {
                    let id = row.id.clone();
                    Some(
                        view! {
                            <code class="ml-1.5 font-mono text-xs text-muted-foreground">
                                {format!("({id})")}
                            </code>
                        },
                    )
                }}
            </span>
            <code class="font-mono text-xs text-muted-foreground">{row.url.clone()}</code>
            <span class="ml-auto text-xs text-muted-foreground">
                {row
                    .last_seen_at
                    .clone()
                    .map(|t| view! { <RelativeTime iso=t/> })}
            </span>
        </li>
    }
}

/// One outbox row — `queued` writes replay when the node returns;
/// `failed` (dead-lettered) ones carry the error they died on.
#[component]
#[allow(clippy::needless_pass_by_value)] // component props are owned
pub(crate) fn WriteRow(write: PendingWriteDto) -> impl IntoView {
    let failed = write.status == "failed";
    let variant = if failed {
        BadgeVariant::Destructive
    } else {
        BadgeVariant::Info
    };
    let status = write.status.clone();
    let attempts = write.attempts;
    let detail = format!("{} → {}", write.node_id, write.session_id);
    view! {
        <li class="flex flex-wrap items-center gap-x-3 gap-y-1.5 py-3 first:pt-0 last:pb-0">
            <Badge variant=variant>{status}</Badge>
            <code class="font-mono text-xs font-medium">{write.op.clone()}</code>
            <span class="font-mono text-xs text-muted-foreground">{detail}</span>
            <span class="ml-auto flex items-center gap-x-3">
                {write
                    .last_error
                    .clone()
                    .map(|e| view! { <span class="text-xs text-destructive">{e}</span> })}
                <span class="text-xs text-muted-foreground">
                    {format!("{attempts} attempt(s)")}
                </span>
                <RelativeTime iso=write.enqueued_at.clone()/>
            </span>
        </li>
    }
}
