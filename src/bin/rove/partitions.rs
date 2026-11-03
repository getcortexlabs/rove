use rove::data::{CompressionError, GapTreatment, Matrix, Patterns};

use super::Json;

#[derive(Clone, Copy)]
pub(super) enum Partitioning {
    Observed,
    Declared,
    Labels,
}

impl Partitioning {
    pub fn parse(value: &str) -> Result<Self, String> {
        match value {
            "observed" => Ok(Self::Observed),
            "declared" => Ok(Self::Declared),
            "labels" => Ok(Self::Labels),
            _ => Err("--partitioning must be observed, declared or labels".into()),
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::Observed => "observed",
            Self::Declared => "declared",
            Self::Labels => "labels",
        }
    }

    pub fn compress(
        self,
        data: &Matrix,
        gaps: GapTreatment,
    ) -> Result<Vec<Patterns>, CompressionError> {
        match self {
            Self::Observed => data.partition_by_states(gaps),
            Self::Labels => data.partition_by_labels(gaps),
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
                let mut fields = vec![
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
                ];
                if p.data_type() == rove::data::DataType::Dna {
                    fields.push(("data_type", Json::text("dna")));
                }
                Json::Object(fields)
            })
            .collect(),
    )
}
