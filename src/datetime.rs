#[cfg(not(any(feature = "time", feature = "chrono")))]
compile_error!("either feature `time` or `chrono` must be enabled");

#[cfg(feature = "time")]
pub type DateTime = time::OffsetDateTime;

#[cfg(all(feature = "chrono", not(feature = "time")))]
pub type DateTime = chrono::DateTime<chrono::Utc>;

#[cfg(feature = "time")]
pub type Duration = time::Duration;

#[cfg(all(feature = "chrono", not(feature = "time")))]
pub type Duration = chrono::Duration;

#[cfg(feature = "time")]
#[must_use]
pub fn now_utc() -> DateTime {
    time::OffsetDateTime::now_utc()
}

#[cfg(all(feature = "chrono", not(feature = "time")))]
#[must_use]
pub fn now_utc() -> DateTime {
    chrono::Utc::now()
}
