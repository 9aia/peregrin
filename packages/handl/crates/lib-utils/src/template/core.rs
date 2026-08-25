use handl_vm::{Context, Function};
use handl_vm_common::Module;
use handl_vm_values_json::{Error, FunctionValue, NonFunctionValue};
use serde_json::Value as JsonValue;

use crate::shared::value_to_string;

// ════════════════════════════════════════════════════════════════════
// Template engine — inspired by lodash-es/template.
//
// Supports:
//   <%= expr %>   interpolated output
//   <%- expr %>   HTML-escaped output
//   <% expr %>    evaluated without output
//
// Options: custom delimiters, imports, and a root variable name.
// ════════════════════════════════════════════════════════════════════

#[derive(Debug, Clone, Copy)]
enum DelimKind {
    Interpolate,
    Escape,
    Evaluate,
}

#[derive(Debug, Clone)]
struct DelimSet {
    interpolate: (String, String),
    escape: (String, String),
    evaluate: (String, String),
}

impl Default for DelimSet {
    fn default() -> Self {
        Self {
            interpolate: ("<%=".to_string(), "%>".to_string()),
            escape: ("<%-".to_string(), "%>".to_string()),
            evaluate: ("<%".to_string(), "%>".to_string()),
        }
    }
}

#[derive(Debug, Clone, Default)]
struct Options {
    delims: DelimSet,
    imports: Vec<(String, NonFunctionValue)>,
    variable: Option<String>,
}

impl Options {
    fn parse(value: Option<&NonFunctionValue>) -> Result<Self, Error> {
        let mut options = Self::default();
        let Some(value) = value else {
            return Ok(options);
        };
        if value.is_null() {
            return Ok(options);
        }
        let obj = value.as_object().ok_or(Error::TypeMismatch {
            expected: "object",
            got: "non-object",
        })?;

        for (k, v) in obj.iter() {
            match k.as_str() {
                "interpolate" => options.delims.interpolate = parse_delim(v)?,
                "escape" => options.delims.escape = parse_delim(v)?,
                "evaluate" => options.delims.evaluate = parse_delim(v)?,
                "imports" => options.imports = parse_imports(v)?,
                "variable" => {
                    options.variable = Some(
                        v.as_str()
                            .ok_or(Error::TypeMismatch {
                                expected: "string",
                                got: "non-string",
                            })?
                            .to_string(),
                    )
                }
                _ => {
                    return Err(Error::PrimitiveError(format!(
                        "unknown template option: {k}"
                    )));
                }
            }
        }

        Ok(options)
    }
}

fn parse_delim(value: &JsonValue) -> Result<(String, String), Error> {
    let arr = value.as_array().ok_or(Error::TypeMismatch {
        expected: "array",
        got: "non-array",
    })?;
    if arr.len() != 2 {
        return Err(Error::PrimitiveError(
            "delimiter must be a [open, close] array".to_string(),
        ));
    }
    let open = arr[0].as_str().ok_or(Error::TypeMismatch {
        expected: "string",
        got: "non-string",
    })?;
    let close = arr[1].as_str().ok_or(Error::TypeMismatch {
        expected: "string",
        got: "non-string",
    })?;
    if open.is_empty() || close.is_empty() {
        return Err(Error::PrimitiveError(
            "delimiter tokens must be non-empty".to_string(),
        ));
    }
    Ok((open.to_string(), close.to_string()))
}

fn parse_imports(value: &JsonValue) -> Result<Vec<(String, NonFunctionValue)>, Error> {
    let obj = value.as_object().ok_or(Error::TypeMismatch {
        expected: "object",
        got: "non-object",
    })?;
    Ok(obj
        .iter()
        .map(|(k, v)| (k.clone(), NonFunctionValue::new(v.clone())))
        .collect())
}

fn build_vars(data: &NonFunctionValue, options: &Options) -> Vec<(String, NonFunctionValue)> {
    let mut vars = options.imports.clone();
    if let Some(var) = &options.variable {
        vars.push((var.clone(), data.clone()));
    } else if let Some(obj) = data.as_object() {
        for (k, v) in obj.iter() {
            vars.push((k.clone(), NonFunctionValue::new(v.clone())));
        }
    } else {
        vars.push(("data".to_string(), data.clone()));
        vars.push(("it".to_string(), data.clone()));
    }
    vars
}

fn html_escape(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for c in input.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#x27;"),
            '/' => out.push_str("&#x2F;"),
            _ => out.push(c),
        }
    }
    out
}

fn render_template(
    template: &str,
    data: &NonFunctionValue,
    options: &Options,
    ctx: &mut dyn Context<NonFunctionValue>,
) -> Result<String, Error> {
    let vars = build_vars(data, options);
    let delims = &options.delims;
    let mut output = String::new();
    let mut i = 0;

    while i < template.len() {
        let mut next: Option<(usize, &str, &str, DelimKind)> = None;

        for (open, close, kind) in [
            (
                delims.interpolate.0.as_str(),
                delims.interpolate.1.as_str(),
                DelimKind::Interpolate,
            ),
            (
                delims.escape.0.as_str(),
                delims.escape.1.as_str(),
                DelimKind::Escape,
            ),
            (
                delims.evaluate.0.as_str(),
                delims.evaluate.1.as_str(),
                DelimKind::Evaluate,
            ),
        ] {
            if let Some(pos) = template[i..].find(open) {
                match next {
                    None => next = Some((pos, open, close, kind)),
                    Some((cur_pos, cur_open, _, _))
                        if pos < cur_pos || (pos == cur_pos && open.len() > cur_open.len()) =>
                    {
                        next = Some((pos, open, close, kind));
                    }
                    _ => {}
                }
            }
        }

        match next {
            Some((pos, open, close, kind)) => {
                let start = i + pos;
                let content_start = start + open.len();
                let rest = &template[content_start..];
                let close_pos = rest.find(close).ok_or_else(|| {
                    Error::PrimitiveError(format!("unclosed template delimiter: {open}"))
                })?;
                let content_end = content_start + close_pos;

                output.push_str(&template[i..start]);
                let expr = template[content_start..content_end].trim();

                if !expr.is_empty() {
                    match kind {
                        DelimKind::Interpolate => {
                            let value = ctx.eval_expr(expr, &vars)?;
                            output.push_str(&value_to_string(value.inner()));
                        }
                        DelimKind::Escape => {
                            let value = ctx.eval_expr(expr, &vars)?;
                            output.push_str(&html_escape(&value_to_string(value.inner())));
                        }
                        DelimKind::Evaluate => {
                            let _ = ctx.eval_expr(expr, &vars)?;
                        }
                    }
                }

                i = content_end + close.len();
            }
            None => {
                output.push_str(&template[i..]);
                break;
            }
        }
    }

    Ok(output)
}

// ── functions ──────────────────────────────────────────────────────

#[derive(Clone, Copy)]
pub struct Render;

impl Function<NonFunctionValue> for Render {
    fn call(
        &self,
        args: &[NonFunctionValue],
        ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, Error> {
        if args.len() < 2 {
            return Err(Error::ArgumentCount {
                expected: 2,
                got: args.len(),
            });
        }
        if args.len() > 3 {
            return Err(Error::ArgumentCount {
                expected: 3,
                got: args.len(),
            });
        }

        let template = args[0].as_str().ok_or(Error::TypeMismatch {
            expected: "string",
            got: "non-string",
        })?;
        let data = &args[1];
        let options = Options::parse(args.get(2))?;
        let rendered = render_template(template, data, &options, ctx)?;
        Ok(NonFunctionValue::new(rendered))
    }
}

#[derive(Clone, Copy)]
pub struct Compile;

impl Function<NonFunctionValue> for Compile {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, Error> {
        if args.is_empty() {
            return Err(Error::ArgumentCount {
                expected: 1,
                got: 0,
            });
        }
        if args.len() > 2 {
            return Err(Error::ArgumentCount {
                expected: 2,
                got: args.len(),
            });
        }

        let template = args[0].as_str().ok_or(Error::TypeMismatch {
            expected: "string",
            got: "non-string",
        })?;

        let options_part = if args.len() == 2 && !args[1].is_null() {
            format!(
                ", {}",
                serde_json::to_string(args[1].inner())
                    .map_err(|e| Error::PrimitiveError(e.to_string()))?
            )
        } else {
            String::new()
        };

        let template_lit = serde_json::to_string(&JsonValue::String(template.to_string()))
            .map_err(|e| Error::PrimitiveError(e.to_string()))?;
        let body = format!("template.render({template_lit}, data{options_part})");

        let fn_val = FunctionValue {
            params: vec![(
                "data".to_string(),
                JsonValue::Object(serde_json::Map::new()),
            )],
            return_schema: None,
            body,
            patterns: vec![],
        };

        Ok(NonFunctionValue::new(fn_val.to_json()))
    }
}

#[derive(Clone, Copy)]
pub struct Escape;

impl Function<NonFunctionValue> for Escape {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, Error> {
        if args.is_empty() {
            return Err(Error::ArgumentCount {
                expected: 1,
                got: 0,
            });
        }
        if args.len() > 1 {
            return Err(Error::ArgumentCount {
                expected: 1,
                got: args.len(),
            });
        }

        let input = args[0].as_str().ok_or(Error::TypeMismatch {
            expected: "string",
            got: "non-string",
        })?;
        Ok(NonFunctionValue::new(html_escape(input)))
    }
}

#[derive(Clone, Copy)]
pub struct TemplateModule;

impl Module for TemplateModule {
    fn name(&self) -> &str {
        "template"
    }

    fn function(&self, name: &str) -> Option<&dyn Function<NonFunctionValue>> {
        match name {
            "render" => Some(&Render),
            "compile" => Some(&Compile),
            "escape" => Some(&Escape),
            _ => None,
        }
    }
}
