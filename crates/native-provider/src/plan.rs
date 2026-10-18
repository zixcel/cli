use crate::{Error, Operation, Prepared, Provider, Request, Result, validation};
use std::collections::BTreeMap;
impl Provider {
    pub fn prepare(&self, request: &Request) -> Result<Prepared> {
        let op = self.operation(request)?;
        let route = self
            .routes
            .operations
            .iter()
            .find(|r| r.id == op.api)
            .ok_or(Error("missing_provider_route"))?;
        let mut path_values = BTreeMap::new();
        for (param, arg) in &op.path_arguments {
            path_values.insert(param.clone(), request.arguments[arg].clone());
        }
        let path = format!(
            "{}{}",
            self.policy.base_path,
            validation::substitute(&route.path, &path_values)?
        );
        let mut url =
            url::Url::parse(&self.policy.origin).map_err(|_| Error("invalid_provider_origin"))?;
        url.set_path(&path);
        if !op.query.is_empty() {
            url.query_pairs_mut().extend_pairs(op.query.iter());
        }
        Ok(Prepared {
            origin: self.policy.origin.clone(),
            url: url.into(),
            headers: op.headers.clone(),
        })
    }
    pub(crate) fn operation(&self, request: &Request) -> Result<&Operation> {
        crate::validate_request(&self.manifest, request)?;
        let op = self
            .policy
            .operations
            .iter()
            .find(|o| o.command.id == request.operation)
            .ok_or(Error("unknown_operation"))?;
        for (name, rule) in &op.argument_rules {
            let value = request
                .arguments
                .get(name)
                .ok_or(Error("missing_argument"))?;
            rule.validate(value)?;
        }
        Ok(op)
    }
}
