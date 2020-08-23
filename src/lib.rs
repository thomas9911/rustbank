//! Yet another CouchDB client
//! 
//! ![couch](https://images.pexels.com/photos/276583/pexels-photo-276583.jpeg?auto=compress&cs=tinysrgb&dpr=3&h=750&w=1260 "Neat Couch")
//! 
//! Rustbank dutch translation of Couch. (and also contains the word rust 🤷)
//! 
//! Build on top of [Reqwest](https://docs.rs/reqwest/latest/reqwest)
//! 


use reqwest::IntoUrl;
use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::Value;

#[derive(Debug)]
pub struct Config {
    pub url: String,
    pub database_name: String,
}

#[derive(Debug)]
pub enum Error {
    Reqwest(reqwest::Error),
    CouchDB(CouchDBError),
    Serde(serde_json::Error),
    Custom(String),
}

impl From<reqwest::Error> for Error {
    fn from(err: reqwest::Error) -> Error {
        Error::Reqwest(err)
    }
}

impl From<serde_json::Error> for Error {
    fn from(err: serde_json::Error) -> Error {
        Error::Serde(err)
    }
}

impl From<CouchDBError> for Error {
    fn from(err: CouchDBError) -> Error {
        Error::CouchDB(err)
    }
}

impl From<String> for Error {
    fn from(err: String) -> Error {
        Error::Custom(err)
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Reqwest(e) => Some(e),
            Error::CouchDB(e) => Some(e),
            Error::Serde(e) => Some(e),
            Error::Custom(_) => None,
        }
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Reqwest(e) => write!(f, "{}", e),
            Error::CouchDB(e) => write!(f, "{}", e),
            Error::Serde(e) => write!(f, "{}", e),
            Error::Custom(e) => write!(f, "{}", e),
        }
    }
}

#[derive(Debug)]
pub struct CouchDBError {
    code: String,
    reason: String,
}

impl std::error::Error for CouchDBError {}

impl CouchDBError {
    pub fn new(code: String, reason: String) -> Self {
        CouchDBError { code, reason }
    }
}

impl std::fmt::Display for CouchDBError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}, reason: {}", self.code, self.reason)
    }
}

pub trait CouchDBObject {
    fn to_id(&self) -> String;
    fn get_id(&self) -> String {
        self.to_id()
    }

    fn get_rev(&self) -> Option<&str>;
    fn has_rev(&self) -> bool {
        self.get_rev().is_some()
    }

    fn update_rev(&mut self, rev: String);
}

fn fetch_value_from_map(map: &serde_json::Map<String, Value>, key: &'static str) -> String {
    map.get(key).unwrap().as_str().unwrap().to_owned()
}

fn to_result<D: DeserializeOwned>(value: Value) -> Result<D, Error> {
    // println!("{}", to_string_pretty(&value).unwrap());
    let xd = if let Value::Object(map) = value {
        if map.contains_key("error") && map.contains_key("reason") {
            Err(CouchDBError::new(
                fetch_value_from_map(&map, "error"),
                fetch_value_from_map(&map, "reason"),
            ))?
        } else {
            Value::Object(map)
        }
    } else {
        value
    };

    Ok(serde_json::from_value(xd)?)
}

pub struct Client {
    client: reqwest::blocking::Client,
    pub config: Config,
}

impl Client {
    pub fn new(config: Config) -> Client {
        Client {
            client: reqwest::blocking::Client::new(),
            config,
        }
    }

    pub fn create_db(&self) -> Result<Value, Error> {
        let res = self.put(&format!(
            "{}/{}",
            self.config.url, self.config.database_name
        ))?;
        Ok(to_result(res)?)
    }

    pub fn delete_db(&self) -> Result<Value, Error> {
        let res = self.delete(&format!(
            "{}/{}",
            self.config.url, self.config.database_name
        ))?;
        Ok(to_result(res)?)
    }

    pub fn put_object<J: Serialize + ?Sized, D: DeserializeOwned>(
        &self,
        body: &J,
    ) -> Result<D, Error> {
        let res = self.post_json(
            &format!("{}/{}", self.config.url, self.config.database_name),
            body,
        )?;
        Ok(to_result(res)?)
    }

    pub fn get_latest_revision(&self, id: &str) -> Result<String, Error> {
        let url = format!("{}/{}/{}", self.config.url, self.config.database_name, id);

        let res = self.head(&url)?;
        match to_result(res) {
            Ok(Value::Object(map)) => {
                let tag_value = map
                    .get("etag")
                    .ok_or(Error::Custom("Invalid etag header".to_string()))?;
                Ok(tag_value.as_str().unwrap().trim_matches('"').to_owned())
            }
            Ok(_) => Err(Error::Custom("Invalid etag header".to_string())),
            Err(e) => Err(e),
        }
    }

    pub fn update_object<J, D>(&self, body: &mut J) -> Result<D, Error>
    where
        J: Serialize + ?Sized + CouchDBObject,
        D: DeserializeOwned,
    {
        let url = format!("{}/{}", self.config.url, self.config.database_name);
        if body.has_rev() {
            let res = self.post_json(&url, body)?;
            Ok(to_result(res)?)
        } else {
            let id = body.get_id();
            let rev = self.get_latest_revision(&id)?;
            body.update_rev(rev);

            self.update_object(body)
        }
    }

    pub fn get_object<D>(&self, id: &str) -> Result<D, Error>
    where
        D: DeserializeOwned,
    {
        let url = format!("{}/{}/{}", self.config.url, self.config.database_name, id);
        let res = self.get(&url)?;
        Ok(to_result(res)?)
    }

    pub fn delete_object<J, D>(&self, body: &mut J) -> Result<D, Error>
    where
        J: Serialize + ?Sized + CouchDBObject,
        D: DeserializeOwned,
    {
        if body.has_rev() {
            let id = body.get_id();
            let rev = body.get_rev().unwrap();

            let url = format!(
                "{}/{}/{}?rev={}",
                self.config.url, self.config.database_name, id, rev
            );

            let res = self.delete(&url)?;
            Ok(to_result(res)?)
        } else {
            let id = body.get_id();
            let rev = self.get_latest_revision(&id)?;
            body.update_rev(rev);

            self.delete_object(body)
        }
    }

    pub fn delete_object_by_id<D>(&self, id: &str) -> Result<D, Error>
    where
        D: DeserializeOwned,
    {
        let rev = self.get_latest_revision(id)?;
        let url = format!(
            "{}/{}/{}?rev={}",
            self.config.url, self.config.database_name, id, rev
        );

        let res = self.delete(&url)?;
        Ok(to_result(res)?)
    }

    // lower level

    pub fn get<U: IntoUrl>(&self, url: U) -> Result<Value, Error> {
        Ok(self.client.get(url).send()?.json()?)
    }

    pub fn head<U: IntoUrl>(&self, url: U) -> Result<Value, Error> {
        let mut map = serde_json::Map::<String, Value>::new();

        for (key, v) in self.client.head(url).send()?.headers().into_iter() {
            if let Ok(value) = v.to_str() {
                map.insert(key.as_str().to_owned(), value.into());
            }
        }

        Ok(Value::Object(map))
    }

    pub fn put<U: IntoUrl>(&self, url: U) -> Result<Value, Error> {
        Ok(self.client.put(url).send()?.json()?)
    }

    pub fn put_json<U, J>(&self, url: U, json: &J) -> Result<Value, Error>
    where
        U: IntoUrl,
        J: Serialize + ?Sized,
    {
        Ok(self.client.put(url).json(json).send()?.json()?)
    }

    pub fn post_json<U, J>(&self, url: U, json: &J) -> Result<Value, Error>
    where
        U: IntoUrl,
        J: Serialize + ?Sized,
    {
        Ok(self.client.post(url).json(json).send()?.json()?)
    }

    pub fn delete<U: IntoUrl>(&self, url: U) -> Result<Value, Error> {
        Ok(self.client.delete(url).send()?.json()?)
    }
}


