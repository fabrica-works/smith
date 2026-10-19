//! Resolve an entered model against startup's endpoint names (shell.md, 4.1).
//! Only the inventory index travels to idle local policy; no connection is composed.
use super::Service;
use alloc::boxed::Box;
use smith_local_domain::Slash;

pub(super) fn resolve(service: &Service, choice: Slash) -> Result<Slash, Box<[u8]>> {
    match choice {
        Slash::Model { model, .. } => {
            let main = service.models().first().expect("startup main model");
            let Some(endpoint) = service.endpoint_name(main.endpoint.0) else {
                return Ok(Slash::Model { model, selected: None });
            };
            model_choice(service.models(), endpoint, model, service.show_bytes())
        }
        choice @ (Slash::Effort { .. } | Slash::New | Slash::Resume { .. } | Slash::Quit) => Ok(choice),
    }
}

fn model_choice(
    models: &[smith_domain::run::charter::Llm],
    endpoint: &[u8],
    model: Box<[u8]>,
    cap: u32,
) -> Result<Slash, Box<[u8]>> {
    let Some(rest) = model.strip_prefix(endpoint) else { return Ok(Slash::Model { model, selected: None }) };
    let Some(name) = rest.strip_prefix(b"/") else { return Ok(Slash::Model { model, selected: None }) };
    for (index, candidate) in models.iter().enumerate() {
        if candidate.endpoint == models.first().expect("startup main model").endpoint
            && candidate.model.as_ref() == name
        {
            return Ok(Slash::Model { model, selected: Some(index) });
        }
    }
    let mut text = skein_lib::List::with_capacity(cap);
    for byte in b"Unknown model ".iter().chain(smith_events::clip(&model)) {
        if text.push(*byte).is_err() {
            break;
        }
    }
    Err(text.into_boxed())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn models() -> [smith_domain::run::charter::Llm; 2] {
        let main = smith_domain::run::charter::Llm {
            effort: None,
            prices: smith_domain::run::Prices { input: 1, cached: 1, cache_write: None, output: 1, unit: 1 },
            dialect: 0,
            account: Some(0),
            endpoint: smith_domain::run::charter::Endpoint(0),
            model: b"first".as_slice().into(),
            window: 8192,
            output: 128,
        };
        let mut other = main.clone();
        other.model = b"other".as_slice().into();
        [main, other]
    }
    #[test]
    fn a_model_of_mains_endpoint_resolves_to_its_startup_position() {
        assert_eq!(
            model_choice(&models(), b"main", b"main/other".as_slice().into(), 1024),
            Ok(Slash::Model { model: b"main/other".as_slice().into(), selected: Some(1) })
        );
    }
    #[test]
    fn an_unknown_model_is_refused_naming_it() {
        assert_eq!(
            model_choice(&models(), b"main", b"main/absent".as_slice().into(), 1024),
            Err(b"Unknown model main/absent".as_slice().into())
        );
    }
    #[test]
    fn another_endpoint_never_resolves_to_a_startup_position() {
        assert_eq!(
            model_choice(&models(), b"main", b"elsewhere/other".as_slice().into(), 1024),
            Ok(Slash::Model { model: b"elsewhere/other".as_slice().into(), selected: None })
        );
    }
}
