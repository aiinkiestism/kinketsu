//! iCalendar (RFC 5545) export of subscription renewal dates.
//!
//! Output is hand-rolled rather than using a third-party crate — the format
//! we need is tiny (VEVENT with DTSTART;VALUE=DATE and an optional RRULE),
//! and avoiding the dependency keeps the build matrix simple.

use chrono::{Datelike, NaiveDate, Timelike, Utc};

use crate::models::{BillingCycle, Subscription, SubscriptionStatus};

fn escape_text(s: &str) -> String {
    // RFC 5545 §3.3.11 — backslash, semicolon, comma, newline require escaping.
    s.replace('\\', "\\\\")
        .replace(';', "\\;")
        .replace(',', "\\,")
        .replace('\n', "\\n")
}

fn rrule_for(cycle: BillingCycle) -> Option<&'static str> {
    match cycle {
        BillingCycle::Weekly => Some("RRULE:FREQ=WEEKLY"),
        BillingCycle::Monthly => Some("RRULE:FREQ=MONTHLY"),
        BillingCycle::Quarterly => Some("RRULE:FREQ=MONTHLY;INTERVAL=3"),
        BillingCycle::SemiAnnual => Some("RRULE:FREQ=MONTHLY;INTERVAL=6"),
        BillingCycle::Annual => Some("RRULE:FREQ=YEARLY"),
        BillingCycle::Custom => None,
    }
}

fn fmt_dtstamp_now() -> String {
    let now = Utc::now();
    format!(
        "{:04}{:02}{:02}T{:02}{:02}{:02}Z",
        now.year(),
        now.month(),
        now.day(),
        now.hour(),
        now.minute(),
        now.second()
    )
}

fn fmt_date(d: NaiveDate) -> String {
    format!("{:04}{:02}{:02}", d.year(), d.month(), d.day())
}

/// Generate an iCalendar document with one VEVENT per active subscription
/// that has a `next_billing_date`. Each event recurs according to the
/// subscription's `billing_cycle` (Custom omits RRULE).
///
/// Cancelled and Paused subscriptions are excluded; Trial is included.
#[must_use]
pub fn export_subscriptions(subs: &[Subscription]) -> String {
    let mut out = String::new();
    out.push_str("BEGIN:VCALENDAR\r\n");
    out.push_str("VERSION:2.0\r\n");
    out.push_str("PRODID:-//kinketsu//Subscription Tracker//EN\r\n");
    out.push_str("CALSCALE:GREGORIAN\r\n");
    out.push_str("METHOD:PUBLISH\r\n");

    let dtstamp = fmt_dtstamp_now();

    for sub in subs {
        if !matches!(
            sub.status,
            SubscriptionStatus::Active | SubscriptionStatus::Trial
        ) {
            continue;
        }
        let Some(date) = sub.next_billing_date else {
            continue;
        };

        let summary = escape_text(&format!("{} renewal", sub.name));
        let plan_part = sub
            .plan
            .as_deref()
            .filter(|s| !s.is_empty())
            .map(|p| format!(" — {p}"))
            .unwrap_or_default();
        let desc = escape_text(&format!(
            "kinketsu: {} {}{}",
            sub.amount_minor, sub.currency, plan_part
        ));

        out.push_str("BEGIN:VEVENT\r\n");
        out.push_str(&format!("UID:{}@kinketsu.app\r\n", sub.id));
        out.push_str(&format!("DTSTAMP:{dtstamp}\r\n"));
        out.push_str(&format!("DTSTART;VALUE=DATE:{}\r\n", fmt_date(date)));
        out.push_str(&format!("SUMMARY:{summary}\r\n"));
        out.push_str(&format!("DESCRIPTION:{desc}\r\n"));
        if let Some(r) = rrule_for(sub.billing_cycle) {
            out.push_str(r);
            out.push_str("\r\n");
        }
        out.push_str("END:VEVENT\r\n");
    }

    out.push_str("END:VCALENDAR\r\n");
    out
}
