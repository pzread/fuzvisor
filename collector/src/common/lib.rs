pub mod fuzzmon_proto {
    tonic::include_proto!("fuzzmon");
}

pub const NO_SANCOV_INDEX: u64 = std::u64::MAX;
