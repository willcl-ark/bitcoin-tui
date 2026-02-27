use std::time::{SystemTime, UNIX_EPOCH};

pub fn fmt_number(n: u64) -> String {
    let s = n.to_string();
    let mut result = String::with_capacity(s.len() + s.len() / 3);
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (s.len() - i).is_multiple_of(3) {
            result.push(',');
        }
        result.push(c);
    }
    result
}

pub fn fmt_weight(n: u64) -> String {
    let n = n as f64;
    if n >= 1_000_000.0 {
        format!("{:.1} MWU", n / 1_000_000.0)
    } else if n >= 1_000.0 {
        format!("{:.0} KWU", n / 1_000.0)
    } else {
        format!("{} WU", n as u64)
    }
}

pub fn fmt_bytes(n: u64) -> String {
    const GB: f64 = 1_073_741_824.0;
    const MB: f64 = 1_048_576.0;
    const KB: f64 = 1_024.0;
    let n = n as f64;
    if n >= GB {
        format!("{:.2} GB", n / GB)
    } else if n >= MB {
        format!("{:.1} MB", n / MB)
    } else if n >= KB {
        format!("{:.0} KB", n / KB)
    } else {
        format!("{} B", n as u64)
    }
}

pub fn fmt_difficulty(d: f64) -> String {
    const E: f64 = 1e18;
    const P: f64 = 1e15;
    const T: f64 = 1e12;
    const G: f64 = 1e9;
    const M: f64 = 1e6;
    if d >= E {
        format!("{:.2} E", d / E)
    } else if d >= P {
        format!("{:.2} P", d / P)
    } else if d >= T {
        format!("{:.2} T", d / T)
    } else if d >= G {
        format!("{:.2} G", d / G)
    } else if d >= M {
        format!("{:.2} M", d / M)
    } else {
        format!("{:.2}", d)
    }
}

pub fn fmt_hashrate(h: f64) -> String {
    const EH: f64 = 1e18;
    const PH: f64 = 1e15;
    const TH: f64 = 1e12;
    const GH: f64 = 1e9;
    const MH: f64 = 1e6;
    if h >= EH {
        format!("{:.1} EH/s", h / EH)
    } else if h >= PH {
        format!("{:.1} PH/s", h / PH)
    } else if h >= TH {
        format!("{:.1} TH/s", h / TH)
    } else if h >= GH {
        format!("{:.1} GH/s", h / GH)
    } else if h >= MH {
        format!("{:.1} MH/s", h / MH)
    } else {
        format!("{:.0} H/s", h)
    }
}

pub fn fmt_sat_per_vb(btc_per_kvb: f64) -> String {
    let sat_per_vb = btc_per_kvb * 100_000.0;
    format!("{:.2} sat/vB", sat_per_vb)
}

pub fn fmt_btc(btc: f64) -> String {
    format!("{:.8} BTC", btc)
}

pub fn fmt_duration(secs: u64) -> String {
    if secs >= 3600 {
        format!("{}h {}m", secs / 3600, (secs % 3600) / 60)
    } else if secs >= 60 {
        format!("{}m {}s", secs / 60, secs % 60)
    } else {
        format!("{}s", secs)
    }
}

pub fn fmt_relative_time(unix: u64) -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    if now > unix {
        format!("{} ago", fmt_duration(now - unix))
    } else {
        "just now".to_string()
    }
}
