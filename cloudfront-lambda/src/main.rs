use chrono::prelude::*;
use deltalake::action::Protocol;
use std::{fs::File, io::Write, collections::HashMap};

use deltalake::*;
use aws_lambda_events::event::s3::S3Event;
use lambda_runtime::{Error, LambdaEvent, run, service_fn};
use tracing::log::trace;

use std::{io::{BufReader, BufRead}};
use flate2::read::GzDecoder;

use tokio_stream::StreamExt;

/// This is the main body for the function.
/// Write your code inside it.
/// There are some code example in the following URLs:
/// - https://github.com/awslabs/aws-lambda-rust-runtime/tree/main/examples
/// - https://github.com/aws-samples/serverless-rust-demo/
async fn handler(
    s3_client: &aws_sdk_s3::Client,
    event: LambdaEvent<S3Event>) -> Result<(), Error> {
    for record in event.payload.records.iter() {
        let mut object = s3_client
            .get_object()
            .bucket(record.s3.bucket.name.as_ref().unwrap())
            .key(record.s3.object.key.as_ref().unwrap())
            .send()
            .await?;

        let mut file = File::create("my-object").expect("create failed");

        let mut byte_count = 0_usize;
        while let Some(bytes) = object.body.try_next().await? {
            let bytes = file.write(&bytes)?;
            // byte_count += bytes;
            trace!("Intermediate write of {bytes}");
        }

        let gz_file = File::open("my-object").expect("Ooops.");
        let compressed_reader = BufReader::new(gz_file);
        let archive = GzDecoder::new(compressed_reader);
        let mut reader_actual = BufReader::new(archive);

        let table_path = Path::from("this_table");
        let maybe_table = deltalake::open_table(&table_path).await;
        let table = match maybe_table {
            Ok(table) => table,
            Err(DeltaTableError::NotATable(_)) => {
                println!("It doesn't look like our delta table has been created");
                // https://github.com/delta-io/delta-rs/blob/main/rust/examples/recordbatch-writer.rs
                create_initialized_table(&table_path).await
            }
            Err(err) => Err(err).unwrap(),
        };

        let mut line = String::new();
        while reader_actual.read_line(&mut line).unwrap() != 0 {
            println!("{line}");
            line.clear();
        }
    }
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        // disable printing the name of the module in every log line.
        .with_target(false)
        // disabling time is handy because CloudWatch will add the ingestion time.
        .without_time()
        .init();

    let config = aws_config::from_env().load().await;
    let s3_client = aws_sdk_s3::Client::new(&config);

    run(service_fn(|event: LambdaEvent<S3Event>| {
        handler(&s3_client, event)
    })).await
}


async fn create_initialized_table(table_path: &Path) -> DeltaTable {
    let mut table = DeltaTableBuilder::from_uri(table_path).build().unwrap();
    let table_schema = WeatherRecord::schema();
    let mut commit_info = serde_json::Map::<String, serde_json::Value>::new();
    commit_info.insert(
        "operation".to_string(),
        serde_json::Value::String("CREATE TABLE".to_string()),
    );
    commit_info.insert(
        "userName".to_string(),
        serde_json::Value::String("test user".to_string()),
    );

    let protocol = Protocol {
        min_reader_version: 1,
        min_writer_version: 1,
    };

    let metadata = DeltaTableMetaData::new(None, None, None, table_schema, vec![], HashMap::new());

    table
        .create(metadata, protocol, Some(commit_info), None)
        .await
        .unwrap();

    table
}

// Creating a simple type alias for improved readability
type Fahrenheit = i32;

struct WeatherRecord {
    timestamp: DateTime<Utc>,
    temp: Fahrenheit,
    lat: f64,
    long: f64,
}

impl WeatherRecord {
    fn schema() -> Schema {
        Schema::new(vec![
            SchemaField::new(
                "timestamp".to_string(),
                SchemaDataType::primitive("timestamp".to_string()),
                true,
                HashMap::new(),
            ),
            SchemaField::new(
                "temp".to_string(),
                SchemaDataType::primitive("integer".to_string()),
                true,
                HashMap::new(),
            ),
            SchemaField::new(
                "lat".to_string(),
                SchemaDataType::primitive("double".to_string()),
                true,
                HashMap::new(),
            ),
            SchemaField::new(
                "long".to_string(),
                SchemaDataType::primitive("double".to_string()),
                true,
                HashMap::new(),
            ),
        ])
    }
}