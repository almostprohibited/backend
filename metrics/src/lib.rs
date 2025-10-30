use std::{env, sync::LazyLock};

use strum_macros::EnumIter;

static CONNECTION_URI: LazyLock<String> = LazyLock::new(|| {
    let host = env::var("PROMETHEUS_HOST").unwrap_or("localhost".into());
    let port = env::var("PROMETHEUS_PORT").unwrap_or("9090".into());

    format!("http://{host}:{port}/api/v1/otlp/v1/metrics")
});

const SERVICE_NAME: &str = "almostprohibited";

#[derive(Debug, EnumIter, Hash, Eq, PartialEq)]
pub enum Metrics {
    /// Counter for firearm product parsed
    CrawledFirearm,
    /// Counter for ammo product parsed
    CrawledAmmunition,
    /// Counter for when ammo failed to parse round count
    CrawledAmmunitionNoRoundCount,
    /// Counter for accessory product parsed
    CrawledOther,
}

impl Metrics {
    fn to_string(&self) -> String {
        match self {
            Metrics::CrawledFirearm => "CRAWLED_FIREARM".to_string(),
            Metrics::CrawledAmmunition => "CRAWLED_AMMUNITION".to_string(),
            Metrics::CrawledOther => "CRAWLED_OTHER".to_string(),
            Metrics::CrawledAmmunitionNoRoundCount => {
                "CRAWLED_AMMUNITION_NO_ROUND_COUNT".to_string()
            }
        }
    }
}

pub mod _private {
    pub use opentelemetry::KeyValue;

    use std::{collections::HashMap, sync::LazyLock, time::Duration};

    use opentelemetry::{
        global,
        metrics::{Counter, Meter},
    };
    use opentelemetry_otlp::{MetricExporter, Protocol, WithExportConfig};
    use opentelemetry_sdk::{
        Resource,
        metrics::{PeriodicReader, SdkMeterProvider},
    };
    use strum::IntoEnumIterator;

    use crate::{CONNECTION_URI, Metrics, SERVICE_NAME};

    static OTEL_METER: LazyLock<Meter> = LazyLock::new(|| {
        global::set_meter_provider(PROVIDER.clone());
        global::meter(SERVICE_NAME)
    });

    pub static PROVIDER: LazyLock<SdkMeterProvider> = LazyLock::new(|| {
        let exporter = MetricExporter::builder()
            .with_http()
            .with_protocol(Protocol::HttpBinary)
            .with_endpoint(CONNECTION_URI.to_string())
            .build()
            .expect("Expect Prometheus exporter to build");

        let reader = PeriodicReader::builder(exporter)
            .with_interval(Duration::from_secs(1))
            .build();

        let resource = Resource::builder().with_service_name(SERVICE_NAME).build();

        SdkMeterProvider::builder()
            .with_reader(reader)
            .with_resource(resource)
            .build()
    });

    pub static COUNTERS: LazyLock<HashMap<Metrics, Counter<u64>>> = LazyLock::new(|| {
        let mut mapping: HashMap<Metrics, Counter<u64>> = HashMap::new();

        for metric in crate::Metrics::iter() {
            let metric_meter = OTEL_METER.u64_counter(metric.to_string()).build();

            mapping.insert(metric, metric_meter);
        }

        mapping
    });
}

#[macro_export]
macro_rules! put_metric {
    ($metric_name:expr, $added_value:expr $(, $key:literal => $value:expr)* $(,)?) => {
        use $crate::_private::{KeyValue, COUNTERS};
        use $crate::Metrics;

        let metric_name: Metrics = $metric_name;
        let added_value: u64 = $added_value;

        let attributes: &[KeyValue] = &[
            $(KeyValue::new($key, $value),)*
        ];

        COUNTERS
            .get(&metric_name)
            .unwrap()
            .add(added_value, attributes);
    };
}
