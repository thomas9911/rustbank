#[macro_use]
extern crate serde_derive;

use serde_json::{to_string_pretty, Value};

use sha2::{Digest, Sha256};
use rustbank::{Client, Config, Error, CouchDBObject};

#[derive(Debug, Serialize, Deserialize)]
pub struct TestObject {
    #[serde(rename = "_id")]
    id: String,
    #[serde(rename = "_rev")]
    #[serde(skip_serializing_if = "Option::is_none")]
    rev: Option<String>,
    name: String,
    fields: Vec<String>,
}

fn sha2(
    input: &str,
) -> sha2::digest::generic_array::GenericArray<u8, <Sha256 as sha2::digest::FixedOutput>::OutputSize>
{
    Sha256::digest(input.as_bytes())
}

fn string_to_uuid(input: &str) -> String {
    format!("{:x}", sha2(input))[..32].to_string()
}

impl TestObject {
    pub fn new(name: String, fields: Vec<String>) -> TestObject {
        let id = string_to_uuid(&name);
        TestObject {
            id,
            name,
            fields,
            rev: None,
        }
    }

    pub fn empty(name: String) -> TestObject {
        TestObject {
            id: String::new(),
            name,
            fields: Vec::new(),
            rev: None,
        }
    }

    // pub fn to_id(&self) -> String {
    //     string_to_uuid(&self.name)
    // }
}

impl CouchDBObject for TestObject {
    fn to_id(&self) -> String {
        string_to_uuid(self.name.as_ref())
    }

    fn get_id(&self) -> String {
        self.id.clone()
    }

    fn get_rev(&self) -> Option<&str> {
        match &self.rev {
            Some(x) => Some(x),
            None => None,
        }
    }

    fn update_rev(&mut self, input: String) {
        self.rev = Some(input);
    }
}

fn action() -> Result<(), Error> {
    let config = Config {
        url: "http://username:password@127.0.0.1:5984".to_string(),
        database_name: "xd".to_string(),
    };

    let client = Client::new(config);

    // client.delete_db().is_ok();

    // let res = client.create_db()?;
    // println!("{}", res);

    // let mut t = TestObject::new("xds".to_string(), vec![String::from("HAHAHAHA")]);
    let t = TestObject::empty("xds".to_string());

    let mut new_t: TestObject = client.get_object(&t.to_id())?;
    println!("{:?}", new_t);

    new_t.fields.push("ha".to_string());

    let res: Value = client.update_object(&mut new_t)?;

    // let res = client.delete_db()?;
    println!("{}", res);

    // println!("{}", t.to_id());

    Ok(())
}

fn main() {
    match action() {
        Ok(_) => (),
        Err(x) => println!("{}", x),
    }
}
