use crate::operations::utils::timestamp::get_current_timestamp_u64;

// Import necessary modules and items
use super::{EXPIRE_TABLE_DEFINITION, Expire};
use log::info;
use redb::{ReadableDatabase, ReadableTable, ReadableTableMetadata};

impl Expire {
    /// Checks if the given `timestamp` has expired.
    ///
    /// This function performs the following steps:
    /// 1. Begins a read transaction to access the expiration table.
    /// 2. Retrieves the expiration time associated with the provided `timestamp`.
    /// 3. Compares the current timestamp with the retrieved expiration time.
    /// 4. If expired, begins a write transaction to remove expired entries.
    /// 5. Logs the deletion of each expired key and the remaining items in the table.
    ///
    /// # Arguments
    ///
    /// * `timestamp` - A `u64` value representing the timestamp to check for expiration.
    ///
    /// # Returns
    ///
    /// * `true` if the `timestamp` has expired or does not exist (already removed).
    /// * `false` if the `timestamp` has not yet expired.
    pub fn expired_check(&self, timestamp: u64) -> bool {
        // Begin a read transaction on the in-memory disk
        let read_transaction = self.in_disk.begin_read().unwrap();

        // Open the expiration table using its definition
        let expire_table = read_transaction
            .open_table(EXPIRE_TABLE_DEFINITION)
            .unwrap();

        // Attempt to retrieve the expiration entry for the given timestamp
        match expire_table
            .get(timestamp)
            .unwrap()
            .and_then(|entry| entry.value())
        {
            // If an expiration time exists and the current time has surpassed it
            Some(expire_time) if get_current_timestamp_u64() > expire_time => {
                // Begin a write transaction to modify the expiration table
                let write_transaction = self.in_disk.begin_write().unwrap();
                {
                    // Open the expiration table for writing
                    let mut write_table = write_transaction
                        .open_table(EXPIRE_TABLE_DEFINITION)
                        .unwrap();

                    // Iterate over all entries in the expiration table
                    for result in expire_table.iter().unwrap() {
                        // Depublic::structure the key-value pair from the result
                        if let Ok((key, _)) = result {
                            let key_timestamp = key.value();
                            // If the key's timestamp is less than or equal to the provided timestamp
                            if key_timestamp <= timestamp {
                                // Remove the expired key from the table
                                write_table.remove(key_timestamp).unwrap();
                                // Log the deletion of the expired key
                                info!("Deleted expired key: {:?}", key_timestamp);
                            }
                        }
                    }

                    // Log the number of items remaining in the expiration table
                    info!(
                        "{} items remaining in expire table",
                        write_table.len().unwrap()
                    );
                }
                // Commit the write transaction to finalize changes
                write_transaction.commit().unwrap();
                // Indicate that the timestamp has expired
                true
            }
            // If an expiration time exists but has not yet expired
            Some(_) => false,
            // Already expired and been removed
            None => true,
        }
    }
}
