# Rust Lambda Workshop

## Intro

- What is Serverless
- What is Lambda
- What is Rust
- Why Lambda + Rust

## Prerequisites

- Docker
- Your own AWS account
- AWS CLI ([https://aws.amazon.com/cli/](https://aws.amazon.com/cli/)) + configuration
- SAM ([https://aws.amazon.com/serverless/sam/](https://aws.amazon.com/serverless/sam/))
- Rust toolchain (recommended [https://rustup.rs/](https://rustup.rs/))
- Zig ([https://ziglang.org/](https://ziglang.org/))
- Cargo Lambda ([https://www.cargo-lambda.info/](https://www.cargo-lambda.info/))

```bash
# Rust
cargo --version
# -> cargo 1.76.0 (c84b36747 2024-01-18)

# Zig (for cross-compiling lambda binaries)
zig version
# -> 0.11.0

# AWS
aws --version
# -> aws-cli/2.15.28 Python/3.11.8 Darwin/23.3.0 exe/x86_64 prompt/off

# AWS login
# you might need to run extra commands to get temporary credentials if you use AWS organizations
# details on how to configure your CLI here: https://docs.aws.amazon.com/cli/latest/userguide/getting-started-quickstart.html
aws sts get-caller-identity
# ->
# {
#     "UserId": "AROATBJTMBXWT2ZAVHYOW:luciano",
#     "Account": "208950529517",
#     "Arn": "arn:aws:sts::208950529517:assumed-role/AWSReservedSSO_AdministratorAccess_d0f4d19d5ba1f39f/luciano"
# }

# Cargo Lambda
cargo lambda --version
# -> cargo-lambda 1.1.0 (e918363 2024-02-19Z)

# SAM
sam --version
# -> SAM CLI, version 1.111.0
```

## Scaffolding

```bash
cargo lambda new ping-it
```

- Not an HTTP function
- EventBridge Event (`eventbridge::EventBridgeEvent`)

## Code overview

- Explain the concept of event-driven
- Explain difference between `main` and `function_handler` and the lifecycle of a Lambda function
- Update handler

```rust
use serde_json::Value; // <--

async fn function_handler(event: LambdaEvent<EventBridgeEvent<Value>>) -> Result<(), Error> {
//                                                           -------
    dbg!(&event); // <--
    Ok(())
}
```

Create example event in `events/eventbridge.json`:

```json
{
    "version": "0",
    "id": "53dc4d37-cffa-4f76-80c9-8b7d4a4d2eaa",
    "detail-type": "Scheduled Event",
    "source": "aws.events",
    "account": "123456789012",
    "time": "2015-10-08T16:53:06Z",
    "region": "us-east-1",
    "resources": [
        "arn:aws:events:us-east-1:123456789012:rule/my-scheduled-rule"
    ],
    "detail": {}
}
```

## Local testing

```bash
cargo lambda watch
```

in another session

```bash
cargo lambda invoke --data-file events/eventbridge.json
```

- Mention list of available examples: [https://github.com/awslabs/aws-lambda-rust-runtime/tree/main/lambda-events/src/fixtures](https://github.com/awslabs/aws-lambda-rust-runtime/tree/main/lambda-events/src/fixtures) (usable with `--data-example`).
- Explain difference between Lambda response (`null`) and logs
- Explain the concept of events
    - Context & Event
    - `LambdaEvent`
    - `Result`
    - AWS events with the `aws_lambda_events` crate (mention feature flags)
    - Mention you can have custom events

## Deployment with Cargo Lambda

```bash
# build
cargo lambda build --release --arm64
# deploy
cargo lambda deploy
```

- login to the web console
- show the lambda was created
- invoke it from the console
- discuss limitations
    - no event! this is not going to be invoked automatically!
        - Show how to setup the event manually
    - We cannot configure the function in other ways (env vars, memory, timeout, etc)
    - it didn’t create a CloudFormation stack, so there are sparse resources that now we have to delete manually:
        - the Lambda
        - a CloudWatch log stream (`/aws/lambda/<function-name>`) with retention set to Never Expire!
        - an IAM role (`cargo-lambda-role-*`)

## Using SAM

- explain the concept of Infrastructure as Code and why it is convenient
- explain what SAM is and how it builds on top of CloudFormation
- mention that SAM integrates well with cargo-lamda so we get the best of both worlds
- when you build APIs, SAM allows you to run a local simulation of API gateway, so you can call your APIs locally
- Create `template.yml`

```yaml
AWSTemplateFormatVersion: 2010-09-09
Transform: AWS::Serverless-2016-10-31

# # Global configuration that is applied by default to specific resources
# Globals:

# # define what's configurable in our stack
# Parameters:
  
# # list all our resources (e.g. lambdas, s3 buckets, etc)
# Resources:
  
# # expose properties of the created resources (e.g. the URL of an API Gateway)
# Outputs:
```

- Every resources follows this structure:

```yaml
ResourceName:
  Type: '<A specific reasource type>' # e.g. AWS::Serverless::Function
  Metadata:
     Key1: Value1
     Key2: Value2
     # ...
  Properties:
     # specific properties depending on the Type
```

- Add definition for our Lambda:

```yaml
Resources:
  
  HealthCheckLambda:
    Type: AWS::Serverless::Function
    Metadata:
      BuildMethod: rust-cargolambda
    Properties:
      CodeUri: .
      Handler: bootstrap
      Runtime: provided.al2023
      Architectures:
        - arm64
      Events:
        ScheduledExecution:
          Type: Schedule
          Properties:
            Schedule: rate(30 minutes)
```

- validate with:

```bash
sam validate --lint
```

- build with

```bash
sam build --beta-features
```

- deploy with

```bash
sam deploy --guided
```

- `--guided` is only needed the first time

- show the web console:
    - CloudFormation stack with all the resources
    - Lambda with configuration (show memory and Timeout

## Making changes

- Let’s change memory and timeout

```yaml
Resources:
  
  HealthCheckLambda:
    # ...
    Properties:
      # ...
      MemorySize: 256
      Timeout: 70
      # ...
```

- To  redeploy (one liner)

```bash
sam validate --lint && sam build --beta-features && sam deploy
```

- Show changes in the web console

## Doing something useful

- Idea: a pingdom-like utility
    - We call an http endpoint every so often and we record
        - response code
        - request time

## Step 1. Making HTTP requests with reqwest

```yaml
cargo add reqwest
```

- reqwest: [https://docs.rs/reqwest/latest/reqwest/](https://docs.rs/reqwest/latest/reqwest/)

- Update handler:

```rust
async fn function_handler(event: LambdaEvent<EventBridgeEvent<Value>>) -> Result<(), Error> {
    let resp = reqwest::get("https://loige.co").await?;
    let status = resp.status().as_u16();
    let success = resp.status().is_success();
    dbg!(status);
    dbg!(success);

    Ok(())
}
```

```rust
sam build --beta-features
```

<aside>
🔥 -- stderr
thread 'main' panicked at /Users/luciano/.cargo/registry/src/index.crates.io-6f17d22bba15001f/openssl-sys-0.9.101/build/find_normal.rs:190:5:

Could not find directory of OpenSSL installation, and this `-sys` crate cannot
proceed without this knowledge. If OpenSSL is installed and this crate had
trouble finding it,  you can set the `OPENSSL_DIR` environment variable for the
compilation process.

Make sure you also have the development packages of openssl installed.
For example, `libssl-dev` on Ubuntu or `openssl-devel` on Fedora.

If you're in a situation where you think the directory *should* be found
automatically, please open a bug at [https://github.com/sfackler/rust-openssl](https://github.com/sfackler/rust-openssl)
and include information about your system as well as this message.

$HOST = aarch64-apple-darwin
$TARGET = aarch64-unknown-linux-gnu
openssl-sys = 0.9.101

</aside>

- **Reqwest**, by default tries to use the system OpenSSL library and when we cross-compile this can be problematic. A more reliable approach is to avoid to do that and use instead a Rust crate that implements TLS:

```toml
# Cargo.toml

# ...
[dependencies]
# ...
reqwest = { version = "0.11.26", default-features = false, features = [
  "rustls-tls",
] }
# ...
```

- Explain briefly what Rust crates feature flags are
- let’s test locally with:

```toml
cargo lambda watch # in a terminal
cargo lambda invoke --data-file events/eventbridge.json # in another
```

- We should see `[src/main.rs:8:5] status = 200`

## Step 2. measure the duration of the request

```rust
// ...
use std::time::Instant;

async fn function_handler(_event: LambdaEvent<EventBridgeEvent<Value>>) -> Result<(), Error> {
    let start = Instant::now(); // <--
    let resp = reqwest::get("https://loige.co").await?;
    let duration = start.elapsed(); // <--

    let status = resp.status().as_u16();
    let success = resp.status().is_success();
    dbg!(status);
    dbg!(success);
    dbg!(duration); // <--

    Ok(())
}
```

## Step 3. Adding a timeout and better error checks

```rust
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()?;

    let resp = client
        .get("https://httpstat.us/504?sleep=60000")
        .send()
        .await?;
```

- For testing `https://httpstat.us/504?sleep=60000`

<aside>
🔥 `cargo lambda invoke --data-file events/eventbridge.json`

```rust
Error: alloc::boxed::Box<dyn core::error::Error + core::marker::Send + core::marker::Sync>

× error sending request for url (https://httpstat.us/504?sleep=60000): operation timed out

Was this error unexpected?
Open an issue in https://github.com/cargo-lambda/cargo-lambda/issues
```

</aside>

- Our entire execution is failing!
- We rather want to capture the error and handle it gracefully

```rust
 let resp = client
        .get("https://httpstat.us/504?sleep=60000")
        .send()
        .await; // <--- removed "?"
    let duration = start.elapsed();

    match resp {
        Ok(resp) => {
            let status = resp.status().as_u16();
            let success = resp.status().is_success();
            dbg!(status);
            dbg!(success);
            dbg!(duration);
        }
        Err(e) => {
            eprintln!("The request failed: {}", e);
        }
    }
```

- Explain the idea of Result type and pattern matching

## Step 4: Making the handler “configurable”

- We want to make the timeout and the URL configurable.
- One way to do that is to use environment variables
- For instance:

```yaml
# template.yml

Resources:
  
  HealthCheckLambda:
    Type: AWS::Serverless::Function
    # ...
    Properties:
      # ...
			Environment:
        Variables:
          URL: 'https://loige.com'
          TIMEOUT: 10
```

- Let’s create a struct to hold our config:

```rust
struct HandlerConfig {
    url: reqwest::Url,
    client: reqwest::Client,
}
```

- explain why we use these types
    - URL for validation
    - client to avoid to recreate a new client per every request. Ideally a client should be created once at init time and reused across invocations.
- Let’s change the signature of the handler:

```rust
async fn function_handler(
    config: &HandlerConfig, // <- now we can receive a reference to the config
    _event: LambdaEvent<EventBridgeEvent<Value>>,
) -> Result<(), Error> {
  // ...
}
```

- the body of the function can now be simplified:

```rust
let start = Instant::now();
		// Here we use the client from config and we don't need to create one
    let resp = config.client.get(config.url.as_str()).send().await; 
    let duration = start.elapsed();

    match resp {
        Ok(resp) => {
            let status = resp.status().as_u16();
            let success = resp.status().is_success();
            dbg!(status);
            dbg!(success);
            dbg!(duration);
        }
        Err(e) => {
            eprintln!("The request failed: {}", e);
        }
    }

    Ok(())
```

- But we need to create this object on init:

```rust
#[tokio::main]
async fn main() -> Result<(), Error> {
    tracing::init_default_subscriber();

		// new code
    let url = env::var("URL").expect("URL environment variable is not set");
    let url = reqwest::Url::parse(&url).expect("URL environment variable is not a valid URL");
    let timeout = env::var("TIMEOUT").unwrap_or_else(|_| "60".to_string());
    let timeout = timeout
        .parse::<u64>()
        .expect("TIMEOUT environment variable is not a valid number");
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(timeout))
        .build()?;
    let config = &HandlerConfig { url, client };
    // end new code

		// updated to pass the config in every invocation
    run(service_fn(move |event| async move {
        function_handler(config, event).await
    }))
    .await
}
```

- testing: now we need to pass the environment variables to our local simulator:

```bash
cargo lambda watch --env-vars URL=https://loige.co,TIMEOUT=5 # one terminal
cargo lambda invoke --data-file events/eventbridge.json # another terminal
```

- Let’s also deploy and test on AWS!

```bash
sam validate --lint && sam build --beta-features && sam deploy
```

- Show environment variables
    - Mention these could be moved to a parameter in the stack to make it more configurable (i.e. you can deploy the same stack multiple times to check different URLs)

## Step 5. Let’s make it even more useful: store data to DynamoDB

- Let’s create the table first
    - We will store data like this:
        - Id: (”URL#Timestamp”) - String (hash key)
        - Timestamp - String(sort key)
        - StatusCode - Number
        - Duration - Number
        - Error - String
        - Success - Boolean

```yaml
Resources:
# ...
  HealthChecksTable:
    Type: AWS::DynamoDB::Table
    DeletionPolicy: Delete
    UpdateReplacePolicy: Delete
    Properties:
      BillingMode: PAY_PER_REQUEST
      KeySchema:
        - AttributeName: "Id"
          KeyType: "HASH"
        - AttributeName: "Timestamp"
          KeyType: "RANGE"
      AttributeDefinitions:
        - AttributeName: "Id"
          AttributeType: "S"
        - AttributeName: "Timestamp"
          AttributeType: "S"
```

- Mention that deletion and update policies are set to delete only because this is a demo. In real-life you probably want `Retain` or `Snapshot`
- We also need to know the name of the table in our Lambda code and have permissions to write in this table:

```yaml
Resources:
# ...
  HealthCheckLambda:
    Type: AWS::Serverless::Function
    # ...
    Properties:
      # ...
      Environment:
        Variables:
          # ...
          TABLE_NAME: !Ref HealthChecksTable # <- new
			# ...
      Policies: # <- new
        - DynamoDBWritePolicy:
            TableName: !Ref HealthChecksTable
```

- Mention that policies is not a native cloudformation construct, but a “shortcut” provided by SAM (serverless policy templates: [https://docs.aws.amazon.com/serverless-application-model/latest/developerguide/serverless-policy-templates.html](https://docs.aws.amazon.com/serverless-application-model/latest/developerguide/serverless-policy-templates.html))
- Let’s deploy that

```bash
sam validate --lint && sam build --beta-features && sam deploy
```

- Show that the table was created and that the name was replicated as an env var into our lambda + the new policy

### Code update

- Let’s now install the Rust SDK for dynamodb (and the generic `aws-config` package

```bash
cargo add aws-config aws-sdk-dynamodb
```

- update config to include `table_name`:

```rust
struct HandlerConfig {
    url: reqwest::Url,
    client: reqwest::Client,
    table_name: String, // <-
}
```

- Add parsing of env var in `main`

```rust
let table_name = env::var("TABLE_NAME").expect("TABLE_NAME environment variable is not set");

let config = &HandlerConfig {
    url,
    client,
    table_name, // <- new
};
```

- We also need to add a DynamoDB client

```rust
struct HandlerConfig {
    url: reqwest::Url,
    client: reqwest::Client,
    table_name: String,
    dynamodb_client: aws_sdk_dynamodb::Client, // <-
}
```

- And we need to initialise that in in our `main`

```rust
let region_provider = RegionProviderChain::default_provider();
let config = aws_config::defaults(BehaviorVersion::latest())
    .region(region_provider)
    .load()
    .await;
let dynamodb_client = aws_sdk_dynamodb::Client::new(&config);

let config = &HandlerConfig {
    url,
    client,
    table_name,
    dynamodb_client, // <-
};
```

- Since we will need to work with datetime and timestamps, it’s convenient to use a library that makes that easy:

```rust
cargo add chrono
```

- Finally we can update our lambda handler code to actually store data into dynamodb

```rust
async fn function_handler(
    config: &HandlerConfig,
    event: LambdaEvent<EventBridgeEvent<Value>>,
) -> Result<(), Error> {
    let start = Instant::now();
    let resp = config.client.get(config.url.as_str()).send().await;
    let duration = start.elapsed();

		// Added logic to get the current timestamp (either from the event or,
		// if not provided, uses the current timestamp)
    let timestamp = event
        .payload
        .time
        .unwrap_or_else(chrono::Utc::now)
        .format("%+")
        .to_string();
    
    // We start to create the record we want to store in DynamoDb
    let mut item = HashMap::new();
    // We insert the Id and the Timestamp fields
    item.insert(
        "Id".to_string(),
        AttributeValue::S(format!("{}#{}", config.url, timestamp)),
    );
    item.insert("Timestamp".to_string(), AttributeValue::S(timestamp));

		// Updated our match statement to populate the record fields
		// depending if the request failed or if it completed
		// Note: we are now returning success: (always false for request failures, 
		// while it depends on the status code for completed requests)
    let success = match resp {
        Ok(resp) => {
            let status = resp.status().as_u16();
            // In case of success we add the Status and the Duration fields
            item.insert("Status".to_string(), AttributeValue::N(status.to_string()));
            item.insert(
                "Duration".to_string(),
                AttributeValue::N(duration.as_millis().to_string()),
            );
            resp.status().is_success()
        }
        Err(e) => {
		        // In case of failure we add the Error field
            item.insert("Error".to_string(), AttributeValue::S(e.to_string()));
            false
        }
    };
    // Finally, we had the Success field
    item.insert("Success".to_string(), AttributeValue::Bool(success));

		// Now we can send the request to DynamoDB
    let insert_result = config
        .dynamodb_client
        .put_item()
        .table_name(config.table_name.as_str())
        .set_item(Some(item))
        .send()
        .await?;

		// And log the result
    tracing::info!("Insert result: {:?}", insert_result);

    Ok(())
}
```

- Deploy and test:

```rust
sam validate --lint && sam build --beta-features && sam deploy
```

- Note for testing: when testing with fake events, be aware you’ll need to change the timestamp in the event manually

**THE END!**

## Ideas for further development of this example

- Make the configuration options stack parameters for more reusability
- Support multiple URLs (could do this concurrently from one lambda execution!)
- Set a TTL to the dynamoDB records so you don’t have to retain them forever (e.g. keep the last 3 months of data)
- Trigger an alarm if the check fails (bonus if your trigger some kind of notification when the site is back online)
- Observability (logs, metrics, traces, etc)
- It could take a snapshot of the content of the page and save it to S3
- Build a nice dashboard like uptime robot
- Turn this into a SaaS (e.g. you might run this lambda from multiple regions to check the availability of a service across regions)