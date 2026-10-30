use rove::dating::{Dating, FossilizedBirthDeath, TipAge};
use serde_json::Value;

pub(super) fn parse(value: &Value, taxa: &[String]) -> Result<Dating, String> {
    let object = value
        .as_object()
        .ok_or("dating settings must be a JSON object")?;
    const KEYS: &[&str] = &[
        "time_unit",
        "birth_rate",
        "death_rate",
        "fossil_sampling_rate",
        "present_sampling",
        "origin",
        "clock_rate",
        "clock_prior_mean",
        "ages",
    ];
    if let Some(key) = object.keys().find(|k| !KEYS.contains(&k.as_str())) {
        return Err(format!("unknown dating setting {key:?}"));
    }
    if value["time_unit"]
        .as_str()
        .is_none_or(|s| s.is_empty() || s.len() > 80 || s.chars().any(char::is_control))
    {
        return Err("time_unit must name the common age and rate unit (for example Ma)".into());
    }
    let number = |key: &str| {
        value[key]
            .as_f64()
            .ok_or_else(|| format!("dating setting {key} needs a number"))
    };
    let ages = value["ages"]
        .as_object()
        .ok_or("ages must map every matrix taxon to an age or [minimum, maximum]")?;
    if ages.len() != taxa.len() || ages.keys().any(|name| !taxa.contains(name)) {
        return Err(
            "ages must name every matrix taxon exactly once, using the parsed matrix labels".into(),
        );
    }
    let ages = taxa
        .iter()
        .map(|name| {
            let a = &ages[name];
            if let Some(age) = a.as_f64() {
                return Ok(TipAge {
                    minimum: age,
                    maximum: age,
                });
            }
            if let Some(pair) = a.as_array().filter(|p| p.len() == 2) {
                if let (Some(minimum), Some(maximum)) = (pair[0].as_f64(), pair[1].as_f64()) {
                    return Ok(TipAge { minimum, maximum });
                }
            }
            Err(format!(
                "invalid age for {name:?}: use a number or [minimum, maximum]"
            ))
        })
        .collect::<Result<Vec<_>, String>>()?;
    let dating = Dating {
        process: FossilizedBirthDeath {
            birth: number("birth_rate")?,
            death: number("death_rate")?,
            fossil_sampling: number("fossil_sampling_rate")?,
            present_sampling: number("present_sampling")?,
            origin: number("origin")?,
        },
        ages,
        clock_rate: number("clock_rate")?,
        clock_prior_mean: if value.get("clock_prior_mean").is_some() {
            Some(number("clock_prior_mean")?)
        } else {
            None
        },
    };
    dating.validate(taxa.len())?;
    Ok(dating)
}
