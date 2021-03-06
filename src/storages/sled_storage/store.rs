use async_trait::async_trait;
use sled::IVec;

use super::{fetch_schema, SledStorage, StorageError};
use crate::try_into;
use crate::{Error, MutResult, Result, Row, RowIter, Schema, Store, StoreMut};

#[async_trait(?Send)]
impl StoreMut<IVec> for SledStorage {
    async fn generate_id(self, table_name: &str) -> MutResult<Self, IVec> {
        let id = try_into!(self, self.tree.generate_id());
        let id = id.to_be_bytes();
        let prefix = format!("data/{}/", table_name);

        let bytes = prefix
            .into_bytes()
            .into_iter()
            .chain(id.iter().copied())
            .collect::<Vec<_>>();

        Ok((self, IVec::from(bytes.as_slice())))
    }

    async fn insert_schema(self, schema: &Schema) -> MutResult<Self, ()> {
        let key = format!("schema/{}", schema.table_name);
        let key = key.as_bytes();
        let value = try_into!(self, bincode::serialize(schema));

        try_into!(self, self.tree.insert(key, value));

        Ok((self, ()))
    }

    async fn delete_schema(self, table_name: &str) -> MutResult<Self, ()> {
        let prefix = format!("data/{}/", table_name);
        let tree = &self.tree;

        for item in tree.scan_prefix(prefix.as_bytes()) {
            let (key, _) = try_into!(self, item);

            try_into!(self, tree.remove(key));
        }

        let key = format!("schema/{}", table_name);
        try_into!(self, tree.remove(key));

        Ok((self, ()))
    }

    async fn insert_data(self, key: &IVec, row: Row) -> MutResult<Self, ()> {
        let value = try_into!(self, bincode::serialize(&row));

        try_into!(self, self.tree.insert(key, value));

        Ok((self, ()))
    }

    async fn delete_data(self, key: &IVec) -> MutResult<Self, ()> {
        try_into!(self, self.tree.remove(key));

        Ok((self, ()))
    }
}

#[async_trait(?Send)]
impl Store<IVec> for SledStorage {
    async fn fetch_schema(&self, table_name: &str) -> Result<Option<Schema>> {
        fetch_schema(&self.tree, table_name).map(|(_, schema)| schema)
    }

    async fn scan_data(&self, table_name: &str) -> Result<RowIter<IVec>> {
        let prefix = format!("data/{}/", table_name);

        let result_set = self.tree.scan_prefix(prefix.as_bytes()).map(move |item| {
            let (key, value) = try_into!(item);
            let value = try_into!(bincode::deserialize(&value));

            Ok((key, value))
        });

        Ok(Box::new(result_set))
    }
}
