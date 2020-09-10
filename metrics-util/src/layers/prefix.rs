use metrics::{Identifier, Key, Recorder};

use crate::layers::Layer;

/// A layer for applying a prefix to every metric key.
pub struct PrefixLayer(String);

impl PrefixLayer {
    /// Creates a new `PrefixLayer` based on the given prefix.
    pub fn new<S: Into<String>>(prefix: S) -> PrefixLayer {
        PrefixLayer(prefix.into())
    }
}

impl<R> Layer<R> for PrefixLayer {
    type Output = Prefix<R>;

    fn layer(&self, inner: R) -> Self::Output {
        Prefix {
            prefix: self.0.clone(),
            inner,
        }
    }
}

/// Applies a prefix to every metric key.
pub struct Prefix<R> {
    prefix: String,
    inner: R,
}

impl<R> Prefix<R> {
    fn prefix_key(&self, key: Key) -> Key {
        key.map_name(|old| format!("{}.{}", self.prefix, old))
    }
}

impl<R: Recorder> Recorder for Prefix<R> {
    fn register_counter(&self, key: Key, description: Option<&'static str>) -> Identifier {
        let new_key = self.prefix_key(key);
        self.inner.register_counter(new_key, description)
    }

    fn register_gauge(&self, key: Key, description: Option<&'static str>) -> Identifier {
        let new_key = self.prefix_key(key);
        self.inner.register_gauge(new_key, description)
    }

    fn register_histogram(&self, key: Key, description: Option<&'static str>) -> Identifier {
        let new_key = self.prefix_key(key);
        self.inner.register_histogram(new_key, description)
    }

    fn increment_counter(&self, key: Key, value: u64) {
        self.inner.increment_counter(key, value);
    }

    fn update_gauge(&self, key: Key, value: f64) {
        self.inner.update_gauge(key, value);
    }

    fn record_histogram(&self, key: Key, value: u64) {
        self.inner.record_histogram(key, value);
    }
}

#[cfg(test)]
mod tests {
    use super::PrefixLayer;
    use crate::debugging::DebuggingRecorder;
    use crate::layers::Layer;
    use metrics::Recorder;

    #[test]
    fn test_basic_functionality() {
        let recorder = DebuggingRecorder::new();
        let snapshotter = recorder.snapshotter();
        let prefix = PrefixLayer::new("testing");
        let layered = prefix.layer(recorder);

        let before = snapshotter.snapshot();
        assert_eq!(before.len(), 0);

        layered.register_counter("counter_metric".into(), None);
        layered.register_gauge("gauge_metric".into(), None);
        layered.register_histogram("histogram_metric".into(), None);

        let after = snapshotter.snapshot();
        assert_eq!(after.len(), 3);

        for (_kind, key, _value) in &after {
            assert!(key.name().starts_with("testing"));
        }
    }
}
