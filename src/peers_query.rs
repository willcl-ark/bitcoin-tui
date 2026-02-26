use std::cmp::Ordering;

use serde_json::Value;

use crate::rpc_types::PeerInfo;

#[derive(Clone, Default)]
pub struct PeerQuery {
    pub filters: Vec<Condition>,
    pub sort: Option<SortSpec>,
}

#[derive(Clone)]
pub struct Condition {
    pub field: String,
    pub op: Op,
    pub value: Literal,
}

#[derive(Clone)]
pub struct SortSpec {
    pub field: String,
    pub descending: bool,
}

#[derive(Clone)]
pub enum Literal {
    Str(String),
    Num(f64),
    Bool(bool),
    Null,
}

#[derive(Clone, Copy)]
pub enum Op {
    Eq,
    Ne,
    Gt,
    Ge,
    Lt,
    Le,
    Contains,
}

pub fn apply(peers: &[PeerInfo], query: &PeerQuery) -> Vec<usize> {
    let rows: Vec<Value> = peers
        .iter()
        .map(|p| serde_json::to_value(p).unwrap_or(Value::Null))
        .collect();

    let mut out: Vec<usize> = (0..peers.len())
        .filter(|&i| query.filters.iter().all(|c| matches_condition(&rows[i], c)))
        .collect();

    if let Some(sort) = &query.sort {
        out.sort_by(|a, b| {
            let va = get_path(&rows[*a], &sort.field);
            let vb = get_path(&rows[*b], &sort.field);
            let ord = compare_values(va, vb);
            if sort.descending { ord.reverse() } else { ord }
        });
    }

    out
}

pub fn get_path<'a>(value: &'a Value, field_path: &str) -> Option<&'a Value> {
    let mut cur = value;
    for part in field_path.split('.') {
        if part.is_empty() {
            return None;
        }
        cur = cur.get(part)?;
    }
    Some(cur)
}

fn matches_condition(value: &Value, cond: &Condition) -> bool {
    let Some(actual) = get_path(value, &cond.field) else {
        return matches!((&cond.op, &cond.value), (Op::Eq, Literal::Null));
    };

    match cond.op {
        Op::Contains => {
            let Some(s) = actual.as_str() else {
                return false;
            };
            let Literal::Str(needle) = &cond.value else {
                return false;
            };
            s.contains(needle)
        }
        Op::Eq | Op::Ne | Op::Gt | Op::Ge | Op::Lt | Op::Le => {
            let ord = compare_literal(actual, &cond.value);
            match cond.op {
                Op::Eq => ord == Some(Ordering::Equal),
                Op::Ne => ord != Some(Ordering::Equal),
                Op::Gt => ord == Some(Ordering::Greater),
                Op::Ge => ord == Some(Ordering::Greater) || ord == Some(Ordering::Equal),
                Op::Lt => ord == Some(Ordering::Less),
                Op::Le => ord == Some(Ordering::Less) || ord == Some(Ordering::Equal),
                Op::Contains => false,
            }
        }
    }
}

fn compare_literal(actual: &Value, rhs: &Literal) -> Option<Ordering> {
    match rhs {
        Literal::Num(n) => actual
            .as_f64()
            .and_then(|a| a.partial_cmp(n))
            .or_else(|| actual.as_str().and_then(|a| a.parse::<f64>().ok()?.partial_cmp(n))),
        Literal::Bool(b) => actual.as_bool().map(|a| a.cmp(b)),
        Literal::Str(s) => {
            if let Some(a) = actual.as_str() {
                Some(a.cmp(s))
            } else if let Some(a) = actual.as_f64() {
                let b = s.parse::<f64>().ok()?;
                a.partial_cmp(&b)
            } else {
                None
            }
        }
        Literal::Null => {
            if actual.is_null() {
                Some(Ordering::Equal)
            } else {
                None
            }
        }
    }
}

fn compare_values(a: Option<&Value>, b: Option<&Value>) -> Ordering {
    match (a, b) {
        (None, None) => Ordering::Equal,
        (None, Some(_)) => Ordering::Greater,
        (Some(_), None) => Ordering::Less,
        (Some(va), Some(vb)) => {
            if let (Some(na), Some(nb)) = (va.as_f64(), vb.as_f64())
                && let Some(ord) = na.partial_cmp(&nb)
            {
                return ord;
            }
            if let (Some(sa), Some(sb)) = (va.as_str(), vb.as_str()) {
                return sa.cmp(sb);
            }
            if let (Some(ba), Some(bb)) = (va.as_bool(), vb.as_bool()) {
                return ba.cmp(&bb);
            }
            va.to_string().cmp(&vb.to_string())
        }
    }
}
