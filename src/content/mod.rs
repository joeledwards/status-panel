use crate::{Dimensions, Frame, config::ContentMode};
use chrono::{Local, NaiveDateTime, Timelike, Utc};
use std::{fs, net::IpAddr, path::Path};

#[derive(Clone, Debug)]
pub struct SystemInfo { pub hostname: String, pub ip: String }

impl SystemInfo {
    pub fn discover() -> Self {
        let hostname = hostname::get().ok().and_then(|h| h.into_string().ok()).unwrap_or_else(|| "--".into());
        let ip = if_addrs::get_if_addrs().ok().and_then(|interfaces| {
            interfaces.into_iter().map(|i| i.ip()).find(|ip| matches!(ip, IpAddr::V4(v4) if !v4.is_loopback()))
        }).map(|ip| ip.to_string()).unwrap_or_else(|| "--".into());
        Self { hostname, ip }
    }
}

pub fn render(
    mode: ContentMode,
    dimensions: Dimensions,
    system_info: &SystemInfo,
) -> anyhow::Result<Frame> {
    match mode {
        ContentMode::Dashboard => Ok(dashboard(dimensions, system_info)),
        ContentMode::Clock => Ok(clock(dimensions)),
        ContentMode::System => Ok(system(dimensions, system_info)),
    }
}

pub fn dashboard(dimensions: Dimensions, info: &SystemInfo) -> Frame {
    let utc = Utc::now();
    dashboard_at(
        dimensions,
        utc.with_timezone(&Local).naive_local(),
        utc.naive_utc(),
        info,
    )
}

fn dashboard_at(
    dimensions: Dimensions,
    local: NaiveDateTime,
    utc: NaiveDateTime,
    info: &SystemInfo,
) -> Frame {
    let local_time = format!("{} < L", local.format("%Y-%m-%d %H:%M"));
    let utc_time = format!("{} < Z", utc.format("%Y-%m-%d %H:%M"));
    let status = format!("{:<15.15} | {:02}", info.ip, utc.second());
    Frame::from_lines(dimensions, [
        local_time.as_str(),
        utc_time.as_str(),
        info.hostname.as_str(),
        status.as_str(),
    ])
}

pub fn clock(dimensions: Dimensions) -> Frame {
    let local = Local::now();
    let utc = Utc::now();
    let (local_text, utc_text) = if dimensions.columns >= 20 {
        (
            format!("{}L", local.format("%Y-%m-%dT%H:%M:%S")),
            format!("{}Z", utc.format("%Y-%m-%dT%H:%M:%S")),
        )
    } else {
        (
            local.format("%Y-%m-%d %H:%M").to_string(),
            utc.format("%Y-%m-%d %H:%M").to_string(),
        )
    };
    Frame::from_lines(dimensions, [
        local_text,
        utc_text,
    ])
}

pub fn system(dimensions: Dimensions, info: &SystemInfo) -> Frame {
    Frame::from_lines(dimensions, [info.hostname.as_str(), info.ip.as_str()])
}

pub fn file_content(dimensions: Dimensions, path: &Path) -> anyhow::Result<Frame> {
    let bytes = fs::read(path)?;
    let text = std::str::from_utf8(&bytes)?;
    Ok(Frame::from_lines(dimensions, text.lines()))
}

pub fn error_frame(dimensions: Dimensions, heading: &str, error: &anyhow::Error) -> Frame {
    let detail = format!("{error:#}");
    Frame::from_lines(dimensions, [heading, detail.as_str(), "using previous config"])
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn renders_system_information() {
        let d = Dimensions::new(8, 2).unwrap();
        let f = system(d, &SystemInfo { hostname: "example-host".into(), ip: "10.0.0.1".into() });
        assert_eq!(f.row(0), b"example-");
        assert_eq!(f.row(1), b"10.0.0.");
    }


    #[test]
    fn dashboard_matches_the_twenty_by_four_layout() {
        let dimensions = Dimensions::new(20, 4).unwrap();
        let local = NaiveDateTime::parse_from_str("2026-10-25 22:02:34", "%Y-%m-%d %H:%M:%S").unwrap();
        let utc = NaiveDateTime::parse_from_str("2026-10-26 04:02:34", "%Y-%m-%d %H:%M:%S").unwrap();
        let info = SystemInfo { hostname: "groot".into(), ip: "192.168.68.55".into() };
        let frame = dashboard_at(dimensions, local, utc, &info);
        assert_eq!(frame.row(0), b"2026-10-25 22:02 < L");
        assert_eq!(frame.row(1), b"2026-10-26 04:02 < Z");
        assert_eq!(frame.row(2), b"groot               ");
        assert_eq!(frame.row(3), b"192.168.68.55   | 34");
    }
}
