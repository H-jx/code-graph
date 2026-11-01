//! 扩展类型由 manifest 登记；公共 DTO 和身份/证据规则始终由 Core 拥有。
use super::collector::PluginManifest;
use super::entity::EntityKind;
use super::error::{CoreError, Issue, codes};
use super::identity::IdentitySlot;
use schemars::JsonSchema;
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::{Map, Value};
use std::collections::{BTreeMap, BTreeSet};

pub const NAME_PATTERN: &str = r"^x\.[A-Za-z0-9_-]+(?:\.[A-Za-z0-9_-]+)*\.[A-Za-z][A-Za-z0-9_-]*$";

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, JsonSchema)]
#[serde(transparent)]
pub struct ExtensionName(#[schemars(regex(pattern = NAME_PATTERN))] String);
impl ExtensionName {
    pub fn new(value: &str) -> Result<Self, CoreError> {
        static PATTERN: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
        if !PATTERN
            .get_or_init(|| regex::Regex::new(NAME_PATTERN).expect("extension name pattern"))
            .is_match(value)
        {
            return Err(CoreError::new(
                codes::SCHEMA_INVALID,
                "extension kind must be x.<pluginId>.<localName>",
            ));
        }
        Ok(Self(value.to_string()))
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
    fn owned_by(&self, plugin: &str) -> bool {
        self.0
            .strip_prefix(&format!("x.{plugin}."))
            .is_some_and(|local| !local.is_empty() && !local.contains('.'))
    }
}
impl<'de> Deserialize<'de> for ExtensionName {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = String::deserialize(deserializer)?;
        Self::new(&value).map_err(serde::de::Error::custom)
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ManifestExtensions {
    #[serde(default)]
    pub entities: Vec<ExtensionEntity>,
    #[serde(default)]
    pub relations: Vec<ExtensionRelation>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ExtensionIdentityComponent {
    pub name: String,
    pub required: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ExtensionEntity {
    pub name: ExtensionName,
    pub identity_components: Vec<ExtensionIdentityComponent>,
    pub attributes_schema: Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ExtensionRelation {
    pub name: ExtensionName,
    pub allowed_from: Vec<EntityKind>,
    pub allowed_to: Vec<EntityKind>,
    pub qualifiers_schema: Value,
}

struct EntityRegistration {
    declaration: ExtensionEntity,
    validator: jsonschema::Validator,
}
struct RelationRegistration {
    declaration: ExtensionRelation,
    validator: jsonschema::Validator,
}
#[derive(Default)]
pub struct ExtensionRegistry {
    entities: BTreeMap<String, EntityRegistration>,
    relations: BTreeMap<String, RelationRegistration>,
}
impl ExtensionRegistry {
    pub fn from_manifests<'a>(
        manifests: impl IntoIterator<Item = &'a PluginManifest>,
    ) -> Result<Self, CoreError> {
        let mut result = Self::default();
        for manifest in manifests {
            result.register(manifest)?;
        }
        Ok(result)
    }
    pub fn names(&self) -> Vec<String> {
        self.entities
            .keys()
            .chain(self.relations.keys())
            .cloned()
            .collect()
    }
    fn register(&mut self, manifest: &PluginManifest) -> Result<(), CoreError> {
        if manifest.extensions.entities.len() + manifest.extensions.relations.len() > 64 {
            return Err(issue(
                "/extensions",
                "at most 64 extension types per manifest",
            ));
        }
        for (index, definition) in manifest.extensions.entities.iter().enumerate() {
            let path = format!("/extensions/entities/{index}");
            self.check_name(&definition.name, &manifest.plugin_id, &path)?;
            let components = &definition.identity_components;
            if components
                .first()
                .is_none_or(|c| c.name != "scopeId" || !c.required)
                || components.iter().any(|c| c.name.is_empty())
                || components
                    .iter()
                    .map(|c| &c.name)
                    .collect::<BTreeSet<_>>()
                    .len()
                    != components.len()
            {
                return Err(issue(
                    &format!("{path}/identityComponents"),
                    "ordered unique identity components must begin with required scopeId",
                ));
            }
            let validator = compile(
                &definition.attributes_schema,
                &format!("{path}/attributesSchema"),
            )?;
            self.entities.insert(
                definition.name.as_str().into(),
                EntityRegistration {
                    declaration: definition.clone(),
                    validator,
                },
            );
        }
        for (index, definition) in manifest.extensions.relations.iter().enumerate() {
            let path = format!("/extensions/relations/{index}");
            self.check_name(&definition.name, &manifest.plugin_id, &path)?;
            if definition.allowed_from.is_empty() || definition.allowed_to.is_empty() {
                return Err(issue(
                    &path,
                    "extension relations must declare allowed endpoint kinds",
                ));
            }
            let validator = compile(
                &definition.qualifiers_schema,
                &format!("{path}/qualifiersSchema"),
            )?;
            self.relations.insert(
                definition.name.as_str().into(),
                RelationRegistration {
                    declaration: definition.clone(),
                    validator,
                },
            );
        }
        Ok(())
    }
    fn check_name(&self, name: &ExtensionName, plugin: &str, path: &str) -> Result<(), CoreError> {
        if !name.owned_by(plugin) {
            return Err(issue(
                &format!("{path}/name"),
                "a plugin may register only its own extension namespace",
            ));
        }
        if self.entities.contains_key(name.as_str()) || self.relations.contains_key(name.as_str()) {
            return Err(issue(
                &format!("{path}/name"),
                "extension kind is already registered",
            ));
        }
        Ok(())
    }
    pub fn validate_entity(
        &self,
        name: &str,
        slots: &[IdentitySlot],
        attributes: &Map<String, Value>,
        path: &str,
    ) -> Result<(), CoreError> {
        let registered = self.entities.get(name).ok_or_else(|| {
            issue(
                &format!("{path}/kind"),
                "extension entity kind is not registered",
            )
        })?;
        let components = &registered.declaration.identity_components;
        if components.len() != slots.len()
            || components
                .iter()
                .zip(slots)
                .any(|(c, s)| c.name != s.name || (c.required && s.value.is_none()))
        {
            return Err(issue(
                &format!("{path}/identity"),
                "identity must match registered ordered components",
            ));
        }
        validate(
            &registered.validator,
            &Value::Object(attributes.clone()),
            &format!("{path}/assertion/attributes"),
        )
    }
    pub fn validate_relation(
        &self,
        name: &str,
        from: &EntityKind,
        to: &EntityKind,
        qualifiers: &Map<String, Value>,
        path: &str,
    ) -> Result<(), CoreError> {
        let registered = self.relations.get(name).ok_or_else(|| {
            issue(
                &format!("{path}/kind"),
                "extension relation kind is not registered",
            )
        })?;
        if !registered.declaration.allowed_from.contains(from)
            || !registered.declaration.allowed_to.contains(to)
        {
            return Err(issue(
                path,
                "extension relation endpoint kinds are not allowed",
            ));
        }
        validate(
            &registered.validator,
            &Value::Object(qualifiers.clone()),
            &format!("{path}/qualifiers"),
        )
    }
}

fn issue(path: &str, hint: &str) -> CoreError {
    CoreError::new(
        codes::SCHEMA_INVALID,
        "invalid extension declaration or value",
    )
    .with_issues(vec![
        Issue::new(codes::SCHEMA_INVALID, path).with_hint(hint),
    ])
}

fn compile(schema: &Value, path: &str) -> Result<jsonschema::Validator, CoreError> {
    if schema.get("type").and_then(Value::as_str) != Some("object") {
        return Err(issue(
            path,
            "extension payload schemas must describe objects",
        ));
    }
    if serde_json::to_vec(schema).expect("JSON schema").len() > 262144 {
        return Err(issue(path, "extension schema exceeds 256 KiB"));
    }
    inspect(schema, path, 0)?;
    let mut schema = schema.clone();
    let object = schema.as_object_mut().expect("object schema");
    let local = object
        .entry("$defs")
        .or_insert_with(|| Value::Object(Map::new()))
        .as_object_mut()
        .ok_or_else(|| issue(path, "$defs must be an object"))?;
    for (name, definition) in bundled_definitions() {
        if local.contains_key(&name) {
            return Err(issue(
                &format!("{path}/$defs/{name}"),
                "extension schemas cannot override bundled Core schemas",
            ));
        }
        local.insert(name, definition);
    }
    jsonschema::validator_for(&schema)
        .map_err(|error| issue(path, &format!("invalid bundled schema: {error}")))
}

fn inspect(schema: &Value, path: &str, depth: usize) -> Result<(), CoreError> {
    if depth > 64 {
        return Err(issue(path, "extension schema nesting exceeds 64 levels"));
    }
    match schema {
        Value::Object(map) => {
            for (key, value) in map {
                let pointer = format!("{path}/{}", key.replace('~', "~0").replace('/', "~1"));
                if key == "$id" || key == "$dynamicRef" || key == "$recursiveRef" {
                    return Err(issue(
                        &pointer,
                        "external schema identifiers and dynamic retrieval are forbidden",
                    ));
                }
                if key == "$ref"
                    && !value
                        .as_str()
                        .is_some_and(|r| r == "#" || r.starts_with("#/$defs/"))
                {
                    return Err(issue(
                        &pointer,
                        "only bundled local $defs references are allowed",
                    ));
                }
                if key == "$schema"
                    && value.as_str() != Some("https://json-schema.org/draft/2020-12/schema")
                {
                    return Err(issue(&pointer, "only JSON Schema 2020-12 is supported"));
                }
                inspect(value, &pointer, depth + 1)?;
            }
        }
        Value::Array(values) => {
            for (i, value) in values.iter().enumerate() {
                inspect(value, &format!("{path}/{i}"), depth + 1)?;
            }
        }
        _ => {}
    }
    Ok(())
}

fn bundled_definitions() -> Map<String, Value> {
    let mut generator = schemars::SchemaGenerator::default();
    let _ = generator.subschema_for::<super::entity::Entity>();
    let _ = generator.subschema_for::<super::relation::Relation>();
    let _ = generator.subschema_for::<super::evidence::Source>();
    let _ = generator.subschema_for::<super::relation::JoinQualifiers>();
    let _ = generator.subschema_for::<super::relation::TransitionQualifiers>();
    generator.definitions().clone()
}

fn validate(
    validator: &jsonschema::Validator,
    value: &Value,
    prefix: &str,
) -> Result<(), CoreError> {
    let issues = validator
        .iter_errors(value)
        .map(|error| {
            let mut path = format!("{prefix}{}", error.instance_path());
            if let jsonschema::error::ValidationErrorKind::Required { property } = error.kind()
                && let Some(property) = property.as_str()
            {
                path.push('/');
                path.push_str(&property.replace('~', "~0").replace('/', "~1"));
            }
            Issue::new(codes::SCHEMA_INVALID, &path)
                .with_expected(error.to_string())
                .with_hint("match the registered extension payload schema")
        })
        .collect::<Vec<_>>();
    if issues.is_empty() {
        Ok(())
    } else {
        Err(
            CoreError::new(codes::SCHEMA_INVALID, "extension payload is invalid")
                .with_issues(issues),
        )
    }
}
