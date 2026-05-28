use chrono::NaiveDate;

use crate::models::Subscription;

/// Generate an iCalendar (RFC 5545) document with renewal-date events for the
/// given subscriptions.
///
/// TODO: implement RFC 5545 emission (candidate library: the `ics` crate).
#[must_use]
pub fn export_subscriptions(_subs: &[Subscription], _as_of: NaiveDate) -> String {
    String::new()
}
