use rove::data::{CompressionError, GapTreatment, Matrix, Patterns};

use super::Json;

#[derive(Clone, Copy)]
pub(super) enum Partitioning {
    Observed,
    Declared,
}

impl Partitioning {
    pub fn parse(value: &str) -> Result<Self, String> {
        match value {
            "observed" => Ok(Self::Observed),
            "declared" => Ok(Self::Declared),
            _ => Err("--partitioning must be observed or declared".into()),
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::Observed => "observed",
            Self::Declared => "declared",
        }
    }

    pub fn compress(
        self,
        data: &Matrix,
        gaps: GapTreatment,
    ) -> Result<Vec<Patterns>, CompressionError> {
        match self {
            Self::Observed => data.partition_by_states(gaps),
            Self::Declared => {
                data.compress(std::slice::from_ref(&(0..data.character_count())), gaps)
            }
        }
    }
}

pub(super) fn describe(partitions: &[Patterns]) -> Json {
    Json::List(
        partitions
            .iter()
            .map(|p| {
                Json::Object(vec![
                    ("ordering", Json::text(p.ordering().name())),
                    ("states", Json::Integer(p.symbols().len() as u64)),
                    ("characters", Json::Integer(p.sites().len() as u64)),
                    ("observed_patterns", Json::Integer(p.weights().len() as u64)),
                    (
                        "sites",
                        Json::List(
                            p.sites()
                                .iter()
                                .map(|s| Json::Integer((s + 1) as u64))
                                .collect(),
                        ),
                    ),
                    (
                        "state_symbols",
                        Json::List(
                            p.state_symbols()
                                .iter()
                                .map(|symbols| Json::text(symbols.iter().collect::<String>()))
                                .collect(),
                        ),
                    ),
                ])
            })
            .collect(),
    )
}
