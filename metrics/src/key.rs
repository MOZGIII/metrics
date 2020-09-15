use crate::{IntoLabels, Label, ScopedString};
use std::{fmt, slice::Iter};

/// A metric key data.
///
/// A key data always includes a name, but can optionally include multiple
/// labels used to further describe the metric.
#[derive(PartialEq, Eq, Hash, Clone, Debug)]
pub struct KeyData {
    name: ScopedString,
    labels: Vec<Label>,
}

impl KeyData {
    /// Creates a `KeyData` from a name.
    pub fn from_name<N>(name: N) -> Self
    where
        N: Into<ScopedString>,
    {
        Self::from_name_and_labels(name, Vec::new())
    }

    /// Creates a `KeyData` from a name and vector of `Label`s.
    pub fn from_name_and_labels<N, L>(name: N, labels: L) -> Self
    where
        N: Into<ScopedString>,
        L: IntoLabels,
    {
        Self {
            name: name.into(),
            labels: labels.into_labels(),
        }
    }

    /// Name of this key.
    pub fn name(&self) -> &ScopedString {
        &self.name
    }

    /// Labels of this key, if they exist.
    pub fn labels(&self) -> Iter<Label> {
        self.labels.iter()
    }

    /// Map the name of this key to a new name, based on `f`.
    ///
    /// The value returned by `f` becomes the new name of the key.
    pub fn map_name<F>(mut self, f: F) -> Self
    where
        F: Fn(ScopedString) -> String,
    {
        let new_name = f(self.name);
        self.name = new_name.into();
        self
    }

    /// Consumes this `Key`, returning the name and any labels.
    pub fn into_parts(self) -> (ScopedString, Vec<Label>) {
        (self.name, self.labels)
    }

    /// Returns a clone of this key with some additional labels.
    pub fn with_extra_labels(&self, extra_labels: Vec<Label>) -> Self {
        if extra_labels.is_empty() {
            return self.clone();
        }

        let name = self.name.clone();
        let mut labels = self.labels.clone();
        labels.extend(extra_labels);

        Self { name, labels }
    }
}

impl fmt::Display for KeyData {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.labels.is_empty() {
            write!(f, "KeyData({})", self.name)
        } else {
            write!(f, "KeyData({}, [", self.name)?;
            let mut first = true;
            for label in &self.labels {
                if first {
                    write!(f, "{} = {}", label.0, label.1)?;
                    first = false;
                } else {
                    write!(f, ", {} = {}", label.0, label.1)?;
                }
            }
            write!(f, "])")
        }
    }
}

impl From<String> for KeyData {
    fn from(name: String) -> Self {
        Self::from_name(name)
    }
}

impl From<&'static str> for KeyData {
    fn from(name: &'static str) -> Self {
        Self::from_name(name)
    }
}

impl<N, L> From<(N, L)> for KeyData
where
    N: Into<ScopedString>,
    L: IntoLabels,
{
    fn from(parts: (N, L)) -> Self {
        Self::from_name_and_labels(parts.0, parts.1)
    }
}

/// Key is used to identiry the metrics in the API calls.
///
/// Key holds either an owned variant or a static ref variant of the KeyData.
/// It's purpose is to allow some flexibility in ways the KeyData can be passed
/// around, enabling performance improvements.
#[derive(Debug, Hash, Clone)]
pub enum Key {
    /// A staticly borrowed KeyData.
    /// If you are capable of keeping a static KeyData around, it's possible
    /// to reduce allocations and improve the performance.
    /// The reference is read-only, so you can't modify the underlying KeyData.
    Borrowed(&'static KeyData),
    /// An owned KeyData.
    /// The plain and simple way of handling KeyData. Useful when you need
    /// to modify a borrowed KeyData in-flight, or when there's no way to
    /// keep around the static KeyData, or when it's undesirable for some
    /// reason.
    Owned(KeyData),
}

impl PartialEq for Key {
    /// We deliberately hide the differences between the containment types.
    fn eq(&self, other: &Self) -> bool {
        self.as_ref() == other.as_ref()
    }
}

impl Eq for Key {}

impl Key {
    /// Converts any kind of [`Key`] into an owned [`KeyData`].
    ///
    /// Owned variant returned as is, borrowed variant is cloned.
    pub fn into_owned(self) -> KeyData {
        match self {
            Self::Borrowed(val) => val.clone(),
            Self::Owned(val) => val,
        }
    }
}

impl std::ops::Deref for Key {
    type Target = KeyData;

    #[must_use]
    fn deref(&self) -> &Self::Target {
        match self {
            Self::Borrowed(val) => val,
            Self::Owned(val) => val,
        }
    }
}

impl AsRef<KeyData> for Key {
    #[must_use]
    fn as_ref(&self) -> &KeyData {
        match self {
            Self::Borrowed(val) => val,
            Self::Owned(val) => val,
        }
    }
}

impl fmt::Display for Key {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Borrowed(val) => val.fmt(f),
            Self::Owned(val) => val.fmt(f),
        }
    }
}

// Here we don't provide generic `From` impls
// (i.e. `impl <T: Into<KeyData>> From<T> for Key`) because the decision whether
// to construct the owned or borrowed ref is important for performance, and
// we want users of this type to explicitly make this decision rather than rely
// on the the magic of `.into()`.

impl From<KeyData> for Key {
    fn from(key_data: KeyData) -> Self {
        Self::Owned(key_data)
    }
}

impl From<&'static KeyData> for Key {
    fn from(key_data: &'static KeyData) -> Self {
        Self::Borrowed(key_data)
    }
}

/// A type to simplify management of the static `KeyData`.
///
/// Allows for an efficient caching of the `KeyData` at the callsites.
pub type OnceKeyData = once_cell::sync::OnceCell<KeyData>;

#[cfg(test)]
mod tests {
    use super::{Key, KeyData, OnceKeyData};
    use crate::Label;

    #[test]
    fn test_key_data_proper_display() {
        let key1 = KeyData::from_name("foobar");
        let result1 = key1.to_string();
        assert_eq!(result1, "KeyData(foobar)");

        let key2 = KeyData::from_name_and_labels("foobar", vec![Label::new("system", "http")]);
        let result2 = key2.to_string();
        assert_eq!(result2, "KeyData(foobar, [system = http])");

        let key3 = KeyData::from_name_and_labels(
            "foobar",
            vec![Label::new("system", "http"), Label::new("user", "joe")],
        );
        let result3 = key3.to_string();
        assert_eq!(result3, "KeyData(foobar, [system = http, user = joe])");

        let key4 = KeyData::from_name_and_labels(
            "foobar",
            vec![
                Label::new("black", "black"),
                Label::new("lives", "lives"),
                Label::new("matter", "matter"),
            ],
        );
        let result4 = key4.to_string();
        assert_eq!(
            result4,
            "KeyData(foobar, [black = black, lives = lives, matter = matter])"
        );
    }

    #[test]
    fn key_equality() {
        let owned_a = KeyData::from_name("a");
        let owned_b = KeyData::from_name("b");

        static STATIC_A: OnceKeyData = OnceKeyData::new();
        static STATIC_B: OnceKeyData = OnceKeyData::new();

        let borrowed_a = STATIC_A.get_or_init(|| owned_a.clone());
        let borrowed_b = STATIC_B.get_or_init(|| owned_b.clone());

        assert_eq!(Key::Owned(owned_a.clone()), Key::Owned(owned_a.clone()));
        assert_eq!(Key::Owned(owned_b.clone()), Key::Owned(owned_b.clone()));

        assert_eq!(Key::Borrowed(borrowed_a), Key::Borrowed(borrowed_a));
        assert_eq!(Key::Borrowed(borrowed_b), Key::Borrowed(borrowed_b));

        assert_eq!(Key::Owned(owned_a.clone()), Key::Borrowed(borrowed_a));
        assert_eq!(Key::Owned(owned_b.clone()), Key::Borrowed(borrowed_b));

        assert_eq!(Key::Borrowed(borrowed_a), Key::Owned(owned_a.clone()));
        assert_eq!(Key::Borrowed(borrowed_b), Key::Owned(owned_b.clone()));

        assert_ne!(Key::Owned(owned_a.clone()), Key::Owned(owned_b.clone()),);
        assert_ne!(Key::Borrowed(borrowed_a), Key::Borrowed(borrowed_b));
        assert_ne!(Key::Owned(owned_a.clone()), Key::Borrowed(borrowed_b));
        assert_ne!(Key::Owned(owned_b.clone()), Key::Borrowed(borrowed_a));
    }
}
