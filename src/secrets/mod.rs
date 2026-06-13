//! Secrets management module.

pub mod manager;
pub mod migration;
pub mod providers;
pub mod storage;
pub mod sync;

pub use manager::SecretManager;
pub use providers::{
    BitwardenSecretManager, GcpSecretManager, JawsSecretManager, OnePasswordSecretManager,
    Provider, SecretRef, detect_providers, select_from_all_providers,
};
#[allow(unused_imports)]
pub use storage::{
    archive_relpath, compute_content_hash, delete_all_archives, delete_working_file,
    get_secret_path, hash_api_ref, load_secret_file, read_working_file, scan_working_dir,
    version_archive_path, working_file_exists, working_file_path, working_relpath,
    write_secret_version, WorkingFile,
};
