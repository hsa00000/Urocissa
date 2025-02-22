use crate::public::{database_struct::database::definition::Database, tree::TREE};
use crate::router::fairing::guard_auth::AuthGuard;
use crate::router::fairing::guard_read_only_mod::ReadOnlyModeGuard;
use rayon::iter::{IntoParallelIterator, ParallelIterator};

#[get("/put/generate_random_data?<number>")]
pub async fn generate_random_data(
    _auth: AuthGuard,
    _read_only_mode: ReadOnlyModeGuard,
    number: usize,
) {
    let data_vec: Vec<Database> = (0..number)
        .into_par_iter()
        .map(|_| Database::generate_random_data())
        .collect();
    TREE.insert_tree_api(&data_vec).unwrap();
    TREE.tree_update();
    info!("Insert random data complete")
}
