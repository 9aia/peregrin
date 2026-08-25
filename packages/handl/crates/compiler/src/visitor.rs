//! Visitor and Folder traits for generic IR traversal.
//!
//! `Visitor` walks an IR tree without mutating — analysis passes
//! (lint, metrics) implement `Visitor`.
//!
//! `Folder` transforms an IR tree, returning a new one — optimization
//! passes implement `Folder`.
//!
//! Both have default implementations that recurse, so passes override
//! only the nodes they care about.

use handl_ast_canonical::{Expr, Field};
use handl_fields::ExtractedFields;
use handl_vm_values_json::NonFunctionValue;

// ---------------------------------------------------------------------------
// Visitor (read-only traversal)
// ---------------------------------------------------------------------------

/// Read-only IR visitor. Override only the methods you care about.
pub trait Visitor {
    fn visit_value(&mut self, _value: &NonFunctionValue) {}

    fn visit_expr(&mut self, expr: &Expr<NonFunctionValue>) {
        walk_expr(self, expr);
    }

    fn visit_field(&mut self, field: &Field<NonFunctionValue>) {
        walk_field(self, field);
    }

    fn visit_extracted(&mut self, extracted: &ExtractedFields) {
        walk_extracted(self, extracted);
    }
}

/// Walk an expression tree with a visitor.
pub fn walk_expr<V: Visitor + ?Sized>(visitor: &mut V, expr: &Expr<NonFunctionValue>) {
    match expr {
        Expr::Value(v) => visitor.visit_value(v),
        Expr::Var(_) => {}
        Expr::Assign { value, .. } => visitor.visit_expr(value),
        Expr::Function { params, ret, .. } => {
            for (_, ty) in params {
                if let Some(ty) = ty {
                    visitor.visit_expr(ty);
                }
            }
            if let Some(ret) = ret {
                visitor.visit_expr(ret);
            }
        }

        Expr::Call { args, .. } => {
            for arg in args {
                visitor.visit_expr(arg);
            }
        }
        Expr::Apply { target, args } => {
            visitor.visit_expr(target);
            for arg in args {
                visitor.visit_expr(arg);
            }
        }
        Expr::Index { target, index } => {
            visitor.visit_expr(target);
            visitor.visit_expr(index);
        }
        Expr::FieldAccess { target, .. } => {
            visitor.visit_expr(target);
        }
        Expr::Handle { body, handlers } => {
            visitor.visit_expr(body);
            for clause in handlers {
                visitor.visit_expr(&clause.body);
            }
        }
        Expr::Array(elements) => {
            for elem in elements {
                visitor.visit_expr(elem);
            }
        }
        Expr::Object(entries) => {
            for (_, expr) in entries {
                visitor.visit_expr(expr);
            }
        }
    }
}

/// Walk a field with a visitor.
pub fn walk_field<V: Visitor + ?Sized>(visitor: &mut V, field: &Field<NonFunctionValue>) {
    visitor.visit_value(&field.value);
}

/// Walk all schemas and data in extracted fields.
pub fn walk_extracted<V: Visitor + ?Sized>(visitor: &mut V, extracted: &ExtractedFields) {
    for (_name, _schema) in &extracted.schemas {
        // Schemas are raw JsonValue — visitors that need schema details
        // should override visit_extracted and inspect directly.
    }
    visitor.visit_value(&extracted.data);
}

// ---------------------------------------------------------------------------
// Folder (transforming traversal)
// ---------------------------------------------------------------------------

/// Transforming IR visitor. Returns new nodes. Override only the
/// methods you care about — defaults recurse and rebuild.
pub trait Folder {
    fn fold_value(&mut self, value: NonFunctionValue) -> NonFunctionValue {
        value
    }

    fn fold_expr(&mut self, expr: Expr<NonFunctionValue>) -> Expr<NonFunctionValue> {
        fold_expr_inner(self, expr)
    }

    fn fold_field(&mut self, field: Field<NonFunctionValue>) -> Field<NonFunctionValue> {
        Field {
            name: field.name,
            value: self.fold_value(field.value),
        }
    }
}

/// Fold an expression tree.
pub fn fold_expr<F: Folder + ?Sized>(
    folder: &mut F,
    expr: Expr<NonFunctionValue>,
) -> Expr<NonFunctionValue> {
    folder.fold_expr(expr)
}

fn fold_expr_inner<F: Folder + ?Sized>(
    folder: &mut F,
    expr: Expr<NonFunctionValue>,
) -> Expr<NonFunctionValue> {
    match expr {
        Expr::Value(v) => Expr::Value(folder.fold_value(v)),
        Expr::Var(name) => Expr::Var(name),
        Expr::Assign { target, value } => Expr::Assign {
            target,
            value: Box::new(folder.fold_expr(*value)),
        },
        Expr::Function {
            type_params,
            params,
            ret,
            body,
        } => Expr::Function {
            type_params,
            params: params
                .into_iter()
                .map(|(p, t)| (p, t.map(|t| Box::new(folder.fold_expr(*t)))))
                .collect(),
            ret: ret.map(|t| Box::new(folder.fold_expr(*t))),
            body,
        },
        Expr::Call {
            name,
            args,
            type_args,
        } => Expr::Call {
            name,
            args: args.into_iter().map(|a| folder.fold_expr(a)).collect(),
            type_args,
        },
        Expr::Apply { target, args } => Expr::Apply {
            target: Box::new(folder.fold_expr(*target)),
            args: args.into_iter().map(|a| folder.fold_expr(a)).collect(),
        },
        Expr::Index { target, index } => Expr::Index {
            target: Box::new(folder.fold_expr(*target)),
            index: Box::new(folder.fold_expr(*index)),
        },
        Expr::FieldAccess { target, field } => Expr::FieldAccess {
            target: Box::new(folder.fold_expr(*target)),
            field,
        },
        Expr::Handle { body, handlers } => Expr::Handle {
            body: Box::new(folder.fold_expr(*body)),
            handlers: handlers
                .into_iter()
                .map(|c| handl_ast_canonical::HandlerClause {
                    op_name: c.op_name,
                    params: c.params,
                    continuation_name: c.continuation_name,
                    body: folder.fold_expr(c.body),
                })
                .collect(),
        },
        Expr::Array(elements) => {
            Expr::Array(elements.into_iter().map(|e| folder.fold_expr(e)).collect())
        }
        Expr::Object(entries) => Expr::Object(
            entries
                .into_iter()
                .map(|(k, v)| (k, folder.fold_expr(v)))
                .collect(),
        ),
    }
}

/// Fold a field.
pub fn fold_field<F: Folder + ?Sized>(
    folder: &mut F,
    field: Field<NonFunctionValue>,
) -> Field<NonFunctionValue> {
    folder.fold_field(field)
}
