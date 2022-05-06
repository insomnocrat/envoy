#[cfg(test)]
mod tests;

use crate::rest::error::SomeError;
use crate::rest::Error;
use crate::rest::ErrorKind::Interpreter as Kind;
use crate::rest::Result;
use convert_case::{self, Case, Casing};
use serde_json::{self, Map, Value};
use std::io::{BufWriter, Write};

const BOOL: &str = "bool";
const STRING: &str = "String";
const NUMBER: &str = "i64";
const UNSIGNED: &str = "u64";
const STRINGARRAY: &str = "Vec<String>";
const NUMARRAY: &str = "Vec<i32>";
const UNARRAY: &str = "Vec<i32>";
const FLOATARRAY: &str = "Vec<f64>";
const OBJARRAY: &str = "Xobjarray";
const SUBMODEL: &str = "Xsubmodel";
const FLOAT: &str = "f64";

const MODELS: &str = "models";

const RESERVED: [&str; 58] = [
    "as",
    "use",
    "extern crate",
    "break",
    "const",
    "continue",
    "crate",
    "else",
    "if",
    "if let",
    "enum",
    "extern",
    "false",
    "fn",
    "for",
    "if",
    "impl",
    "in",
    "for",
    "let",
    "loop",
    "match",
    "mod",
    "move",
    "mut",
    "pub",
    "impl",
    "ref",
    "return",
    "Self",
    "self",
    "static",
    "struct",
    "super",
    "trait",
    "true",
    "type",
    "unsafe",
    "use",
    "where",
    "while",
    "abstract",
    "alignof",
    "become",
    "box",
    "do",
    "final",
    "macro",
    "offsetof",
    "override",
    "priv",
    "proc",
    "pure",
    "sizeof",
    "typeof",
    "unsized",
    "virtual",
    "yield",
];

pub type Fields = std::collections::HashMap<String, Field>;
pub type Models = std::collections::HashMap<String, Model>;
pub type SubModels = std::collections::HashMap<String, Map<String, Value>>;

#[derive(Debug, Default)]
pub struct Interpreter {
    config: Config,
}

impl Interpreter {
    pub fn new(config: Config) -> Self {
        Self { config }
    }

    pub fn change_opt_field_proc(&mut self, proc: OptionalFieldProc) {
        self.config.optional_field_procedure = proc;
    }

    pub fn change_mod_structure(&mut self, proc: ModStructure) {
        self.config.mod_structure = proc;
    }

    pub fn write_response(
        &self,
        response_name: Option<&str>,
        fields: Fields,
        sub_models: Models,
    ) -> Result<()> {
        let response_name = response_name.unwrap_or("Response");
        let mut objects = vec![(
            response_name.to_string(),
            self.format_object(response_name, fields),
        )];
        sub_models
            .into_iter()
            .for_each(|(n, sm)| objects.push((n.to_string(), self.format_object(&n, sm.fields))));
        for (name, object) in objects.iter() {
            self.write_response_to_file(object, Some(name))?;
        }
        #[cfg(test)]
        {
            if let ModStructure::OneModFolder(_folder) = &self.config.mod_structure {
                std::fs::remove_dir_all(format!("src/{_folder}"))
                    .map_err(|e| Error::new("could not remove folder", Kind, e.some_box()))?;
            } else {
                for (name, _object) in objects.iter() {
                    self.remove_file(name)
                        .map_err(|e| Error::new("Could not remove file", Kind, e.some_box()))?;
                }
            }
        }

        Ok(())
    }

    pub fn format_object(&self, name: &str, fields: Fields) -> String {
        let _name = name.to_case(Case::Pascal);
        let _properties = fields
            .iter()
            .map(|(_n, f)| self.format_field(f))
            .collect::<Vec<String>>()
            .join("");

        format!( "\n#[derive(Clone,Debug,Serialize,Deserialize)]\npub struct {_name} {{{_properties}\n}}")
    }

    pub fn read_response(&self, response: Value, response_name: Option<&str>) -> Result<()> {
        let (mut model, mut competing_sub_models) = match response {
            Value::Array(a) => self.read_response_array(a),
            Value::Object(o) => (self.read_response_object(o), Vec::new()),
            _ => panic!("response invalid"),
        };
        let sub_models = self.compare_competing_submodels(&mut model, &mut competing_sub_models);
        self.write_response(response_name, model.fields, sub_models)
    }

    fn compare_competing_submodels(&self, main: &mut Model, competing: &mut Vec<Models>) -> Models {
        let mut sub_models = Models::new();
        let mfs = &mut main.fields;
        let msm = &mut main.sub_models;
        if competing.is_empty() && !msm.is_empty() {
            for (key, model) in msm {
                sub_models.insert(key.to_string(), self.read_model(&model));
            }
            return sub_models;
        }
        for csm in competing.iter_mut() {
            for (key, model) in csm {
                let existing = msm.get_mut(key);
                match existing {
                    Some(existing) => {
                        let mut sub_model = self.read_model(&existing);
                        self.compare_competing_models(model, &mut sub_model);
                    }
                    None => {
                        mfs.insert(
                            key.to_string(),
                            Field {
                                name: key.to_string(),
                                value: FieldValue {
                                    kind: SUBMODEL.to_string(),
                                    is_optional: true,
                                },
                            },
                        );
                        sub_models.insert(key.to_string(), model.clone());
                    }
                }
            }
        }

        sub_models
    }

    fn compare_competing_models(&self, model: &mut Model, competing_model: &mut Model) {
        for (key, competing_field) in competing_model.fields.iter_mut() {
            let same = model.fields.get_mut(key);
            if let Some(existing) = same {
                match (
                    competing_field.value.is_optional,
                    existing.value.is_optional,
                ) {
                    (true, true) | (false, false) => (),
                    (false, true) => {
                        existing.value.kind = competing_field.value.kind.to_string();
                    }
                    (true, false) => {
                        existing.value.is_optional = true;
                    }
                }
            } else {
                model.fields.insert(
                    key.to_string(),
                    Field {
                        name: key.to_string(),
                        value: FieldValue {
                            kind: competing_field.value.kind.to_string(),
                            is_optional: true,
                        },
                    },
                );
            }
        }
    }

    pub fn read_response_array(&self, response: Vec<Value>) -> (Model, Vec<Models>) {
        let mut response = response.iter();
        let first = response.next().expect("empty response");
        let mut model = Model {
            fields: Fields::new(),
            sub_models: SubModels::new(),
        };
        let mut competing_sub_models: Vec<Models> = Vec::new();
        match first {
            Value::Object(o) => self.read_to_model(&mut model, o),
            _ => {
                model.save_field("singular", first);
                return (model, competing_sub_models);
            }
        }
        while let Some(field) = response.next() {
            let mut competing_model = Model::default();
            match field {
                Value::Object(o) => self.read_to_model(&mut competing_model, o),
                _ => {
                    model.save_field("singular", first);
                    return (model, competing_sub_models);
                }
            }
            let sub_models = self.read_sub_models(&mut competing_model);
            self.compare_competing_models(&mut model, &mut competing_model);
            competing_sub_models.push(sub_models);
        }

        (model, competing_sub_models)
    }

    fn read_sub_models(&self, model: &mut Model) -> Models {
        let mut sub_models = Models::new();
        for (name, value) in model.sub_models.iter() {
            let mut sub_model = Model::default();
            self.read_to_model(&mut sub_model, value);
            sub_models.insert(name.to_string(), sub_model);
        }

        sub_models
    }

    fn read_model(&self, obj: &Map<String, Value>) -> Model {
        let mut model = Model::default();
        self.read_to_model(&mut model, obj);

        model
    }

    fn read_to_model(&self, model: &mut Model, obj: &Map<String, Value>) {
        for (key, value) in obj.iter() {
            model.save_field(key, value);
        }
    }
    pub fn read_response_object(&self, response: Map<String, Value>) -> Model {
        self.read_model(&response)
    }

    fn get_file_name(&self, response_name: Option<&str>) -> Result<String> {
        let with_extension = |folder: &str, incoming: &str| {
            let out = incoming.to_case(Case::Snake);
            match out.ends_with(".rs") {
                true => format!("{folder}/{out}"),
                false => format!("{folder}/{out}.rs"),
            }
        };
        Ok(match &self.config.mod_structure {
            ModStructure::OneSrcFile(file) => with_extension("src", file),
            ModStructure::OneModFolder(_folder) => with_extension(
                &format!("src/{_folder}"),
                response_name.ok_or_else(|| {
                    Error::new("OneModFolder requires named response", Kind, None)
                })?,
            ),
            ModStructure::IndividualSrcFiles => with_extension(
                "src",
                response_name.ok_or_else(|| {
                    Error::new("IndividualSrcFiles requires named response", Kind, None)
                })?,
            ),
        })
    }

    fn write_response_to_file(&self, response: &str, response_name: Option<&str>) -> Result<()> {
        let name = self.get_file_name(response_name)?;
        let mut writer = self.create_models_structure(&name)?;

        writer
            .write_all(response.as_bytes())
            .map_err(|e| Error::new("could not write response to file", Kind, e.some_box()))
    }

    #[cfg(test)]
    fn remove_file(&self, name: &str) -> Result<()> {
        let name = self.get_file_name(Some(name))?;
        if !std::path::Path::new(&name).exists() {
            return Ok(());
        }
        std::fs::remove_file(&name)
            .map_err(|e| Error::new("could not delete file", Kind, e.some_box()))
    }

    fn create_models_structure(&self, name: &str) -> Result<BufWriter<std::fs::File>> {
        let writer = match std::path::Path::new(name).exists() {
            true => {
                let file = std::fs::File::options()
                    .append(true)
                    .open(&name)
                    .map_err(|e| Error::new("could not open file", Kind, e.some_box()))?;
                BufWriter::new(file)
            }
            false => {
                if let ModStructure::OneModFolder(_folder) = &self.config.mod_structure {
                    std::fs::create_dir_all(format!("src/{_folder}")).map_err(|e| {
                        Error::new("could not create model directory", Kind, e.some_box())
                    })?;
                }
                let file = std::fs::File::options()
                    .write(true)
                    .create(true)
                    .open(&name)
                    .map_err(|e| Error::new("could not open file", Kind, e.some_box()))?;
                let mut buf_writer = BufWriter::new(file);
                buf_writer
                    .write_all("use serde::{Deserialize, Serialize};\n".as_bytes())
                    .map_err(|e| Error::new("could not create file", Kind, e.some_box()))?;
                buf_writer
            }
        };

        Ok(writer)
    }

    pub fn format_field(&self, field: &Field) -> String {
        let mut name = field.name.to_string();
        let mut aliases = Vec::with_capacity(6);
        if !is_snake_case(&field.name) {
            aliases.push(name.to_string());
            name = name.to_case(Case::Snake);
        }
        self.config.field_aliases.iter().for_each(|c| {
            let cased = name.to_case(c.clone());
            if !name.eq(&cased) {
                aliases.push(cased);
            }
        });
        if RESERVED.iter().any(|k| k.eq(&name)) {
            aliases.push(name.clone());
            name.pop();
        }
        let mut o = self.format_optional(field, &name);
        if !aliases.is_empty() {
            let _alias_string = aliases
                .iter()
                .map(|a| format!("alias = \"{a}\""))
                .collect::<Vec<String>>()
                .join(",");
            o = format!("\n\t#[serde({_alias_string})]{o}")
        }

        o
    }

    fn format_optional(&self, field: &Field, formatted_name: &str) -> String {
        let submodel_name = || formatted_name.to_case(Case::Pascal);
        let _keyword = match field.value.kind.as_str() {
            SUBMODEL => submodel_name(),
            OBJARRAY => format!("Vec<{}>", submodel_name()),
            _ => field.value.kind.to_string(),
        };
        let optional = || {
            format!("\n\t#[serde(skip_serializing_if = \"Option::is_none\")]\n\tpub {formatted_name}: Option<{_keyword}>,")
        };
        let format = |default: bool| match (field.value.is_optional, default) {
            (true, false) => optional(),
            (true, true) => {
                format!("\n\t#[serde(default)]\n\tpub {formatted_name}: Option<{_keyword}>,")
            }
            (_, _) => format!("\n\tpub {formatted_name}: {_keyword},"),
        };
        match self.config.optional_field_procedure {
            OptionalFieldProc::CaseByCase => format(false),
            OptionalFieldProc::AllDefault => format(true),
            OptionalFieldProc::AllOptional => optional(),
        }
    }
}

#[derive(Debug)]
pub struct Config {
    field_aliases: Vec<Case>,
    optional_field_procedure: OptionalFieldProc,
    mod_structure: ModStructure,
}

impl Config {
    pub fn opt_field_proc(self, proc: OptionalFieldProc) -> Self {
        Self {
            field_aliases: self.field_aliases,
            optional_field_procedure: proc,
            mod_structure: self.mod_structure,
        }
    }

    pub fn mod_structure(self, mod_structure: ModStructure) -> Self {
        Self {
            field_aliases: self.field_aliases,
            optional_field_procedure: self.optional_field_procedure,
            mod_structure,
        }
    }

    pub fn alias_fields(self, cases: Vec<Case>) -> Self {
        Self {
            field_aliases: cases,
            optional_field_procedure: self.optional_field_procedure,
            mod_structure: self.mod_structure,
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            field_aliases: vec![],
            optional_field_procedure: OptionalFieldProc::default(),
            mod_structure: ModStructure::default(),
        }
    }
}
#[derive(Debug)]
pub enum OptionalFieldProc {
    CaseByCase,
    AllOptional,
    AllDefault,
}

impl Default for OptionalFieldProc {
    fn default() -> Self {
        Self::CaseByCase
    }
}
#[derive(Debug)]
pub enum ModStructure {
    OneSrcFile(String),
    OneModFolder(String),
    IndividualSrcFiles,
}

impl Default for ModStructure {
    fn default() -> Self {
        Self::OneSrcFile(MODELS.to_string())
    }
}

#[derive(Default, Debug, Clone)]
pub struct Model {
    fields: Fields,
    sub_models: SubModels,
}

impl Model {
    fn save_field(&mut self, key: &str, value: &Value) {
        match value {
            Value::Object(o) => {
                self.save_nested_obj(key, o.to_owned());
                return;
            }
            _ => {
                let fv = self.read_value(key, value);
                self.fields.insert(
                    key.to_string(),
                    Field {
                        name: key.to_string(),
                        value: fv,
                    },
                );
            }
        }
    }
    fn read_value(&mut self, key: &str, value: &Value) -> FieldValue {
        match value {
            Value::Bool(_) => FieldValue {
                kind: BOOL.to_string(),
                is_optional: false,
            },
            Value::String(_) => FieldValue {
                kind: STRING.to_string(),
                is_optional: false,
            },
            Value::Number(_) => {
                let kind: String;
                if value.is_f64() {
                    kind = FLOAT.to_string();
                } else if value.is_u64() {
                    kind = UNSIGNED.to_string();
                } else {
                    kind = NUMBER.to_string();
                }
                FieldValue {
                    kind,
                    is_optional: false,
                }
            }
            Value::Array(v) => self.read_array(key, v),
            Value::Object(o) => {
                self.save_nested_obj(key, o.to_owned());
                FieldValue {
                    kind: SUBMODEL.to_string(),
                    is_optional: false,
                }
            }
            Value::Null => FieldValue {
                kind: STRING.to_string(),
                is_optional: true,
            },
        }
    }
    fn read_array(&mut self, key: &str, values: &Vec<Value>) -> FieldValue {
        let mut values = values.iter();
        let mut value = match values.next() {
            Some(value) => self.read_value(key, value),
            None => {
                return FieldValue {
                    kind: STRINGARRAY.to_string(),
                    is_optional: true,
                }
            }
        };
        match value.kind.as_str() {
            STRING => value.kind = STRINGARRAY.to_string(),
            NUMBER => value.kind = NUMARRAY.to_string(),
            FLOAT => value.kind = FLOATARRAY.to_string(),
            UNSIGNED => value.kind = UNARRAY.to_string(),
            SUBMODEL => value.kind = OBJARRAY.to_string(),
            _ => (),
        }

        value
    }

    fn save_nested_obj(&mut self, father: &str, obj: Map<String, Value>) {
        self.sub_models.insert(father.to_string(), obj);
    }
}

#[derive(Default, Debug, Clone)]
pub struct Field {
    name: String,
    value: FieldValue,
}

#[derive(Default, Debug, Clone)]
struct FieldValue {
    kind: String,
    is_optional: bool,
}

fn is_snake_case(s: &str) -> bool {
    !s.chars().any(|c| c.is_whitespace() || c.is_uppercase())
}
