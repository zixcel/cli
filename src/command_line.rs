use crate::{
    Error, Result, discovery, execution, extensions,
    registry::{self, Registry},
    select,
};
pub async fn run(args: &[String]) -> Result<bool> {
    if args.len() > 64
        || args
            .iter()
            .any(|a| a.len() > 4096 || a.bytes().any(|b| b.is_ascii_control()))
    {
        return Err(Error("invalid_arguments"));
    }
    let normalized = crate::confirmation::leading(args);
    let args = normalized.as_slice();
    if args == ["version"] || args == ["--version"] {
        println!("zix {}", env!("CARGO_PKG_VERSION"));
        return Ok(true);
    }
    if args.first().is_some_and(|a| a == "api") {
        return crate::api_command::run(&args[1..]).await;
    }
    if args.first().is_some_and(|a| a == "credential") {
        return crate::credential::run(&args[1..]).await;
    }
    let path = registry::location()?;
    if args.first().is_some_and(|a| a == "extensions") {
        extensions::run(&path, &args[1..])?;
        return Ok(true);
    }
    let registry = Registry::load(&path)?;
    if args.first().is_some_and(|a| a == "capabilities") && args.iter().any(|a| a == "--compact") {
        let remaining = args[1..]
            .iter()
            .filter(|a| a.as_str() != "--compact")
            .map(String::as_str)
            .collect::<Vec<_>>();
        if args.iter().filter(|a| a.as_str() == "--compact").count() != 1
            || !(remaining.is_empty() || remaining == ["--format", "json"])
        {
            return Err(Error("invalid_arguments"));
        }
        println!("{}", discovery::compact_capabilities(&registry)?);
        return Ok(true);
    }
    if args.is_empty() || ["help", "--help", "list", "capabilities"].contains(&args[0].as_str()) {
        if args.len() > 1 && args[1..] != ["--format", "json"] {
            return Err(Error("invalid_arguments"));
        }
        if args.first().is_some_and(|a| a == "capabilities") || args.len() > 1 {
            println!("{}", discovery::capabilities(&registry)?);
        } else {
            println!("{}", discovery::help(&registry)?);
        }
        return Ok(true);
    }
    if args
        .first()
        .is_some_and(|a| a == "describe" || a == "prompt")
    {
        if args.len() >= 2
            && args[0] == "describe"
            && args[2..]
                .iter()
                .any(|a| a == "--operation" || a == "--guidance")
        {
            let filtered = args
                .iter()
                .filter(|a| a.as_str() != "--guidance")
                .cloned()
                .collect::<Vec<_>>();
            crate::guidance::describe(&registry, &filtered).await?;
            return Ok(true);
        }
        if args[0] == "prompt" && args.len() != 2 {
            return Err(Error("invalid_arguments"));
        }
        if args.len() != 2 && !(args.len() == 4 && args[2..] == ["--format", "json"]) {
            return Err(Error("invalid_arguments"));
        }
        let entry = registry.entry(&args[1])?;
        if args[0] == "describe" && args.len() == 4 {
            println!(
                "{}",
                serde_json::to_string(&entry.manifest()?).map_err(|_| Error("invalid_manifest"))?
            );
        } else {
            println!("{}", discovery::prompt(&registry, &args[1])?);
        }
        return Ok(true);
    }
    let entry = registry.entry(&args[0])?;
    let m = entry.manifest()?;
    if args[1..] == ["--help"] {
        println!("{}", discovery::prompt(&registry, &args[0])?);
        return Ok(true);
    }
    let selected = select(&m, &args[1..])?;
    let mut response = execution::dispatch(entry, selected).await?;
    if let Err(error) = crate::guidance::enrich(&registry, entry, &mut response) {
        response["diagnostic"]["resolution_error"] = serde_json::json!(error.0);
    }
    let success = response.get("error").is_none()
        && response
            .pointer("/diagnostic/state")
            .and_then(serde_json::Value::as_str)
            != Some("NeedsResolution");
    println!("{response}");
    Ok(success)
}
