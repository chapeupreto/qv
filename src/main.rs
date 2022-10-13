use clap::Parser;
use datafusion::common::Result;
use datafusion::prelude::*;
use std::env;

mod args;
mod globbing_path;
mod globbing_table;
mod object_store_util;

use crate::args::Args;
use crate::globbing_path::GlobbingPath;
use crate::globbing_table::build_table_provider;
use crate::object_store_util::register_object_store;

#[tokio::main]
async fn main() -> Result<()> {
    let sdk_config = aws_config::load_from_env().await;

    let config = SessionConfig::new().with_information_schema(true);
    let ctx = SessionContext::with_config(config);

    let args = Args::parse();
    set_aws_profile_when_needed(&args);
    let globbing_path = args.get_globbing_path(&sdk_config).await?;
    register_object_store(&sdk_config, &ctx, &globbing_path.object_store_url).await?;

    let table_arc = build_table_provider(&ctx, &globbing_path, &args.at).await?;
    ctx.register_table("tbl", table_arc)?;

    let query = &args.get_query();
    let df = ctx.sql(query).await?;
    df.show_limit(args.limit).await?;

    Ok(())
}

fn set_aws_profile_when_needed(args: &Args) {
    if let Some(aws_profile) = &args.profile {
        env::set_var("AWS_PROFILE", aws_profile);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use assert_cmd::prelude::*;
    use std::process::Command;
    use predicates::prelude::*;
    use datafusion::common::DataFusionError;

    #[tokio::test]
    async fn run_without_file_exits_with_usage() -> Result<()> {
        let mut cmd = Command::cargo_bin("qv")
            .map_err(|e| DataFusionError::External(Box::new(e)))?;
        cmd.assert().failure().stderr(predicate::str::contains("Usage: qv <PATH>"));
        Ok(())
    }

    #[tokio::test]
    async fn run_with_local_avro_file() -> Result<()> {
        let mut cmd = Command::cargo_bin("qv")
            .map_err(|e| DataFusionError::External(Box::new(e)))?;
        //let cmd = cmd.arg("/Users/timvw/src/github/qv/testing/data/avro/alltypes_plain.avro");
        let cmd = cmd.arg("./testing/data/avro/alltypes_plain.avro");
        cmd.assert().success()
            .stdout(predicate::str::contains("| id | bool_col | tinyint_col | smallint_col | int_col | bigint_col | float_col | double_col | date_string_col  | string_col | timestamp_col       |"))
            .stdout(predicate::str::contains("| 4  | true     | 0           | 0            | 0       | 0          | 0         | 0          | 30332f30312f3039 | 30         | 2009-03-01 00:00:00 |"));
        Ok(())
    }
}
