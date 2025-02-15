use std::borrow::Cow;
use std::fmt::{Display, Formatter};
use std::net::{Ipv4Addr, Ipv6Addr};
use std::ops::{Deref, DerefMut};

use tantivy::schema::FacetParseError;
use time::formatting::Formattable;
use time::OffsetDateTime;

use crate::wrappers::{Bytes, Text};
use crate::{FieldType, UserDisplayType};

pub type KeyValues<'a> = Vec<(Cow<'a, str>, Value<'a>)>;
pub type KeyValuesIter<'a> = std::vec::IntoIter<(Cow<'a, str>, Value<'a>)>;

#[derive(Debug, Clone, Default)]
/// A JSON-like document object.
///
/// Internally this is a vector of tuples
/// which can potentially contain duplicate keys.
pub struct DynamicDocument<'a>(pub KeyValues<'a>);

impl<'a> From<KeyValues<'a>> for DynamicDocument<'a> {
    fn from(value: KeyValues<'a>) -> Self {
        Self(value)
    }
}

impl<'a> Deref for DynamicDocument<'a> {
    type Target = KeyValues<'a>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'a> DerefMut for DynamicDocument<'a> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<'a> UserDisplayType for KeyValues<'a> {
    fn type_name(&self) -> Cow<'static, str> {
        Cow::Borrowed("object")
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Value<'a> {
    Null,
    Str(Cow<'a, str>),
    U64(u64),
    I64(i64),
    F64(f64),
    Bool(bool),
    Facet(Facet<'a>),
    DateTime(DateTime),
    IpAddr(Ipv6Addr),
    Bytes(Vec<u8>),
    Array(Vec<Value<'a>>),
    Object(KeyValues<'a>),
}

impl<'a> Value<'a> {
    #[inline]
    pub fn as_field_type(&self) -> FieldType {
        match self {
            Value::Null => FieldType::Null,
            Value::Str(_) => FieldType::String,
            Value::U64(_) => FieldType::U64,
            Value::I64(_) => FieldType::I64,
            Value::F64(_) => FieldType::F64,
            Value::Bool(_) => FieldType::Bool,
            Value::Facet(_) => FieldType::Facet,
            Value::DateTime(_) => FieldType::DateTime,
            Value::IpAddr(_) => FieldType::IpAddr,
            Value::Bytes(_) => FieldType::Bytes,
            Value::Array(_) => FieldType::Array,
            Value::Object(_) => FieldType::Object,
        }
    }
}

impl<'a> UserDisplayType for Value<'a> {
    fn type_name(&self) -> Cow<'static, str> {
        match self {
            Value::Null => Cow::Borrowed("null"),
            Value::Str(_) => Cow::Borrowed("string"),
            Value::U64(_) => Cow::Borrowed("u64"),
            Value::I64(_) => Cow::Borrowed("i64"),
            Value::F64(_) => Cow::Borrowed("f64"),
            Value::Bool(_) => Cow::Borrowed("bool"),
            Value::Facet(_) => Cow::Borrowed("facet"),
            Value::DateTime(_) => Cow::Borrowed("datetime"),
            Value::Bytes(_) => Cow::Borrowed("bytes"),
            Value::Array(_) => Cow::Borrowed("array"),
            Value::Object(_) => Cow::Borrowed("object"),
            Value::IpAddr(_) => Cow::Borrowed("ip"),
        }
    }
}

impl<'a> UserDisplayType for &Value<'a> {
    fn type_name(&self) -> Cow<'static, str> {
        match self {
            Value::Null => Cow::Borrowed("null"),
            Value::Str(_) => Cow::Borrowed("string"),
            Value::U64(_) => Cow::Borrowed("u64"),
            Value::I64(_) => Cow::Borrowed("i64"),
            Value::F64(_) => Cow::Borrowed("f64"),
            Value::Bool(_) => Cow::Borrowed("bool"),
            Value::Facet(_) => Cow::Borrowed("facet"),
            Value::DateTime(_) => Cow::Borrowed("datetime"),
            Value::Bytes(_) => Cow::Borrowed("bytes"),
            Value::Array(_) => Cow::Borrowed("array"),
            Value::Object(_) => Cow::Borrowed("object"),
            Value::IpAddr(_) => Cow::Borrowed("ip"),
        }
    }
}

impl<'a> From<&'a str> for Value<'a> {
    fn from(value: &'a str) -> Self {
        Self::Str(Cow::Borrowed(value))
    }
}

impl<'a> From<String> for Value<'a> {
    fn from(value: String) -> Self {
        Self::Str(Cow::Owned(value))
    }
}

impl<'a> From<Text<'a>> for Value<'a> {
    fn from(value: Text<'a>) -> Self {
        Self::Str(value.into())
    }
}

impl<'a> From<Cow<'a, str>> for Value<'a> {
    fn from(value: Cow<'a, str>) -> Self {
        Self::Str(value)
    }
}

impl<'a> From<Bytes> for Value<'a> {
    fn from(value: Bytes) -> Self {
        Self::Bytes(value.0)
    }
}

impl<'a> From<u64> for Value<'a> {
    fn from(value: u64) -> Self {
        Self::U64(value)
    }
}

impl<'a> From<i64> for Value<'a> {
    fn from(value: i64) -> Self {
        Self::I64(value)
    }
}

impl<'a> From<f64> for Value<'a> {
    fn from(value: f64) -> Self {
        Self::F64(value)
    }
}

impl<'a> From<bool> for Value<'a> {
    fn from(value: bool) -> Self {
        Self::Bool(value)
    }
}

impl<'a> From<Ipv4Addr> for Value<'a> {
    fn from(value: Ipv4Addr) -> Self {
        Self::IpAddr(value.to_ipv6_mapped())
    }
}

impl<'a> From<Ipv6Addr> for Value<'a> {
    fn from(value: Ipv6Addr) -> Self {
        Self::IpAddr(value)
    }
}

impl<'a> From<DateTime> for Value<'a> {
    fn from(value: DateTime) -> Self {
        Self::DateTime(value)
    }
}

impl<'a> From<tantivy::schema::Facet> for Value<'a> {
    fn from(value: tantivy::schema::Facet) -> Self {
        Self::Facet(value.into())
    }
}

impl<'a> From<Facet<'a>> for Value<'a> {
    fn from(value: Facet<'a>) -> Self {
        Self::Facet(value)
    }
}

impl<'a, T: Into<Value<'a>>> From<Vec<T>> for Value<'a> {
    fn from(value: Vec<T>) -> Self {
        Self::Array(value.into_iter().map(|v| v.into()).collect())
    }
}

impl<'a, T: Into<Value<'a>>> From<Vec<(Cow<'a, str>, T)>> for Value<'a> {
    fn from(value: Vec<(Cow<'a, str>, T)>) -> Self {
        Self::Object(value.into_iter().map(|(k, v)| (k, v.into())).collect())
    }
}

impl<'a> From<DynamicDocument<'a>> for Value<'a> {
    fn from(value: DynamicDocument<'a>) -> Self {
        Self::Object(value.0)
    }
}

#[derive(Debug, Clone, PartialEq)]
/// A facet value which uses borrowed data.
///
/// This is just a new-type wrapper to make it
/// clearer and easier to use; It performs no validation.
pub struct Facet<'a>(pub(crate) Cow<'a, str>);

impl<'a> Facet<'a> {
    #[inline]
    /// Consumes the facet and returns the inner Cow.
    pub fn into_cow(self) -> Cow<'a, str> {
        self.0
    }

    #[inline]
    /// Converts the facet to a tantivy type.
    pub fn to_tantivy_facet(&self) -> Result<tantivy::schema::Facet, FacetParseError> {
        tantivy::schema::Facet::from_text(self.0.as_ref())
    }

    #[inline]
    /// Returns the facet as a string.
    pub fn as_str(&self) -> &str {
        self.0.as_ref()
    }
}

impl<'a> Display for Facet<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl<'a> From<Cow<'a, str>> for Facet<'a> {
    fn from(value: Cow<'a, str>) -> Self {
        Self(value)
    }
}

impl<'a> From<&'a str> for Facet<'a> {
    fn from(value: &'a str) -> Self {
        Self(Cow::Borrowed(value))
    }
}

impl<'a> From<String> for Facet<'a> {
    fn from(value: String) -> Self {
        Self(Cow::Owned(value))
    }
}

impl<'a> From<tantivy::schema::Facet> for Facet<'a> {
    fn from(value: tantivy::schema::Facet) -> Self {
        Self(Cow::Owned(value.to_path_string()))
    }
}

impl<'a> UserDisplayType for Facet<'a> {
    fn type_name(&self) -> Cow<'static, str> {
        Cow::Borrowed("facet")
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Ord, PartialOrd, Eq)]
/// The datatime value used by lnx.
///
/// This is a lightweight wrapper around a `i64` timestamp with `microsecond`  precision.
pub struct DateTime {
    micros: i64,
}

impl DateTime {
    /// The maximum timestamp supported.
    pub const MAX: DateTime = DateTime { micros: i64::MAX };
    /// The minimum timestamp supported.
    pub const MIN: DateTime = DateTime { micros: i64::MIN };

    #[inline]
    /// Converts a given timestamp in seconds to a datetime value.
    ///
    /// This can return `None` if the value cannot be safely represented by the datetime.
    pub fn from_secs(v: i64) -> Option<Self> {
        Self::from_millis(i64::checked_mul(v, 1000)?)
    }

    #[inline]
    /// Converts a given timestamp in milliseconds to a datetime value.
    ///
    /// This can return `None` if the value cannot be safely represented by the datetime.
    pub fn from_millis(v: i64) -> Option<Self> {
        Self::from_micros(i64::checked_mul(v, 1000)?)
    }

    #[inline]
    /// Converts a given timestamp in microseconds to a datetime value.
    ///
    /// This can never return `None`, it's just marked as such for consistency and future proofing.
    pub fn from_micros(v: i64) -> Option<Self> {
        Some(Self { micros: v })
    }

    #[inline]
    /// Formats the datetime into a given format.
    pub fn format(
        &self,
        format: &(impl Formattable + ?Sized),
    ) -> anyhow::Result<String> {
        OffsetDateTime::from_unix_timestamp_nanos(self.micros as i128 * 1000)
            .map_err(|_| anyhow::anyhow!("Cannot format datetime as is beyond what the format supports rendering"))?
            .format(format)
            .map_err(|e| anyhow::anyhow!("Cannot format datetime with the given format: {e}"))
    }

    #[inline]
    /// Gets the timestamp representation of the datetime with microsecond precision.
    pub fn as_micros(&self) -> i64 {
        self.micros
    }

    #[inline]
    /// Gets the datetime as a tantivy-compatible datetime.
    pub fn as_tantivy_value(&self) -> tantivy::DateTime {
        tantivy::DateTime::from_timestamp_micros(self.micros)
    }
}
