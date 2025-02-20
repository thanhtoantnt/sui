// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::schema_v2::packages;
use diesel::ExpressionMethods;
use diesel::QueryDsl;
use diesel::RunQueryDsl;
use move_core_types::language_storage::ModuleId;
use move_core_types::resolver::ModuleResolver;
use sui_types::base_types::ObjectID;
use sui_types::move_package::MovePackage;

use crate::errors::{Context, IndexerError};
use crate::models_v2::packages::StoredPackage;
use crate::store::diesel_marco::read_only_blocking;
use crate::PgConnectionPool;

/// A package resolver that reads packages from the database.
pub struct IndexerStoreModuleResolver {
    cp: PgConnectionPool,
}

impl IndexerStoreModuleResolver {
    // TODO remove this after integration is done
    #[allow(dead_code)]
    pub fn new(cp: PgConnectionPool) -> Self {
        Self { cp }
    }
}

impl ModuleResolver for IndexerStoreModuleResolver {
    type Error = IndexerError;

    fn get_module(&self, id: &ModuleId) -> Result<Option<Vec<u8>>, Self::Error> {
        let package_id = ObjectID::from(*id.address()).to_vec();
        let module_name = id.name().to_string();

        // Note: this implementation is potentially vulnerable to package upgrade race conditions
        // for framework packages because they reuse the same package IDs.
        let stored_package: StoredPackage = read_only_blocking!(&self.cp, |conn| {
            packages::dsl::packages
                .filter(packages::dsl::package_id.eq(package_id))
                .first::<StoredPackage>(conn)
        })
        .context("Error reading module.")?;

        let move_package =
            bcs::from_bytes::<MovePackage>(&stored_package.move_package).map_err(|e| {
                IndexerError::PersistentStorageDataCorruptionError(format!(
                    "Error deserializing move package. Error: {}",
                    e
                ))
            })?;

        Ok(move_package
            .serialized_module_map()
            .get(&module_name)
            .cloned())
    }
}
