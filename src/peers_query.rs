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

pub fn apply_command(query: &mut PeerQuery, input: &str) -> Result<(), String> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Ok(());
    }

    let lower = trimmed.to_ascii_lowercase();
    if lower == "clear" {
        *query = PeerQuery::default();
        return Ok(());
    }

    if lower == "clear where" {
        query.filters.clear();
        return Ok(());
    }

    if lower == "clear sort" {
        query.sort = None;
        return Ok(());
    }

    if lower.starts_with("where ") || lower == "where" {
        let body = trimmed.get(5..).unwrap_or_default().trim();
        if body.is_empty() {
            query.filters.clear();
            return Ok(());
        }

        let clauses = split_and_clauses(body);
        let mut filters = Vec::with_capacity(clauses.len());
        for clause in clauses {
            filters.push(parse_condition(&clause)?);
        }
        query.filters = filters;
        return Ok(());
    }

    if lower.starts_with("sort ") {
        let body = trimmed[5..].trim();
        if body.is_empty() {
            return Err("sort requires a field path".to_string());
        }
        let parts: Vec<&str> = body.split_whitespace().collect();
        if parts.is_empty() || parts.len() > 2 {
            return Err("sort syntax: sort <field> [asc|desc]".to_string());
        }
        let descending = match parts.get(1).map(|s| s.to_ascii_lowercase()) {
            None => false,
            Some(dir) if dir == "asc" => false,
            Some(dir) if dir == "desc" => true,
            Some(_) => return Err("sort direction must be asc or desc".to_string()),
        };
        query.sort = Some(SortSpec {
            field: parts[0].to_string(),
            descending,
        });
        return Ok(());
    }

    Err("unknown command: use where/sort/clear".to_string())
}

pub fn summary(query: &PeerQuery) -> String {
    if is_empty(query) {
        return "none".to_string();
    }

    let mut parts = Vec::new();
    if !query.filters.is_empty() {
        let clauses: Vec<String> = query.filters.iter().map(format_condition).collect();
        parts.push(format!("where {}", clauses.join(" and ")));
    }
    if let Some(sort) = &query.sort {
        parts.push(format!(
            "sort {} {}",
            sort.field,
            if sort.descending { "desc" } else { "asc" }
        ));
    }
    parts.join(" | ")
}

pub fn is_empty(query: &PeerQuery) -> bool {
    query.filters.is_empty() && query.sort.is_none()
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

fn format_condition(c: &Condition) -> String {
    format!(
        "{} {} {}",
        c.field,
        match c.op {
            Op::Eq => "==",
            Op::Ne => "!=",
            Op::Gt => ">",
            Op::Ge => ">=",
            Op::Lt => "<",
            Op::Le => "<=",
            Op::Contains => "~=",
        },
        format_literal(&c.value)
    )
}

fn format_literal(v: &Literal) -> String {
    match v {
        Literal::Str(s) => format!("{:?}", s),
        Literal::Num(n) => n.to_string(),
        Literal::Bool(b) => b.to_string(),
        Literal::Null => "null".to_string(),
    }
}

fn split_and_clauses(input: &str) -> Vec<String> {
    let chars: Vec<char> = input.chars().collect();
    let mut out = Vec::new();
    let mut start = 0usize;
    let mut i = 0usize;
    let mut quote: Option<char> = None;

    while i < chars.len() {
        let c = chars[i];
        if c == '\'' || c == '"' {
            if quote == Some(c) {
                quote = None;
            } else if quote.is_none() {
                quote = Some(c);
            }
            i += 1;
            continue;
        }

        if quote.is_none() && i + 4 < chars.len() {
            let is_sep = chars[i] == ' '
                && chars[i + 1].eq_ignore_ascii_case(&'a')
                && chars[i + 2].eq_ignore_ascii_case(&'n')
                && chars[i + 3].eq_ignore_ascii_case(&'d')
                && chars[i + 4] == ' ';
            if is_sep {
                out.push(chars[start..i].iter().collect::<String>().trim().to_string());
                start = i + 5;
                i += 5;
                continue;
            }
        }
        i += 1;
    }

    out.push(chars[start..].iter().collect::<String>().trim().to_string());
    out.into_iter().filter(|s| !s.is_empty()).collect()
}

fn parse_condition(clause: &str) -> Result<Condition, String> {
    let candidates = ["==", "!=", ">=", "<=", "~=", ">", "<"];
    let mut found: Option<(usize, &str)> = None;
    for op in candidates {
        if let Some(idx) = find_outside_quotes(clause, op) {
            found = Some((idx, op));
            break;
        }
    }

    let (idx, op) =
        found.ok_or_else(|| "where clause needs operator (== != > >= < <= ~=)".to_string())?;
    let left = clause[..idx].trim();
    let right = clause[idx + op.len()..].trim();

    if left.is_empty() || right.is_empty() {
        return Err("where clause must be: <field> <op> <value>".to_string());
    }

    Ok(Condition {
        field: left.to_string(),
        op: match op {
            "==" => Op::Eq,
            "!=" => Op::Ne,
            ">" => Op::Gt,
            ">=" => Op::Ge,
            "<" => Op::Lt,
            "<=" => Op::Le,
            "~=" => Op::Contains,
            _ => unreachable!(),
        },
        value: parse_literal(right),
    })
}

fn find_outside_quotes(haystack: &str, needle: &str) -> Option<usize> {
    let bytes = haystack.as_bytes();
    let needle_bytes = needle.as_bytes();
    let mut quote: Option<u8> = None;
    let mut i = 0usize;

    while i + needle_bytes.len() <= bytes.len() {
        let b = bytes[i];
        if b == b'\'' || b == b'"' {
            if quote == Some(b) {
                quote = None;
            } else if quote.is_none() {
                quote = Some(b);
            }
            i += 1;
            continue;
        }
        if quote.is_none() && &bytes[i..i + needle_bytes.len()] == needle_bytes {
            return Some(i);
        }
        i += 1;
    }
    None
}

fn parse_literal(raw: &str) -> Literal {
    let s = raw.trim();
    if s.len() >= 2
        && ((s.starts_with('"') && s.ends_with('"')) || (s.starts_with('\'') && s.ends_with('\'')))
    {
        return Literal::Str(s[1..s.len() - 1].to_string());
    }
    match s.to_ascii_lowercase().as_str() {
        "true" => Literal::Bool(true),
        "false" => Literal::Bool(false),
        "null" => Literal::Null,
        _ => s
            .parse::<f64>()
            .map(Literal::Num)
            .unwrap_or_else(|_| Literal::Str(s.to_string())),
    }
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
