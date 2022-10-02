use daggy::Walker;
use num::Zero;
use secp256k1::SecretKey;
use serde::*;
use serde_json::{Number, Value};
use std::str::FromStr;
use std::time::{SystemTime, UNIX_EPOCH};
use std::{
    collections::HashMap,
    env,
    ops::{Add, Div, Mul, Rem, Sub},
};
use web3::contract::tokens::Tokenize;
use web3::contract::Options;
use web3::types::{Bytes, TransactionParameters, U256};
use web3::{
    contract::Contract,
    types::{Address, H160},
};

#[derive(Debug, Deserialize, Serialize, PartialEq, Clone)]
#[serde()]
enum ZapType {
    #[serde(rename = "ARITHMETIC")]
    Arithmetic,
    #[serde(rename = "ROOT")]
    Root,
    #[serde(rename = "ACTION")]
    Action,
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Clone)]
#[serde()]
enum ActionType {
    #[serde(rename = "SWAP_EXACT_ETH_FOR_TOKENS")]
    SwapExactETHForTokens,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde()]
struct NodeData {
    #[serde(skip_serializing_if = "Option::is_none")]
    left: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    right: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    operator: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    token_from_address: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    token_to_address: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    token_from_amount: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    action_type: Option<ActionType>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde()]
pub struct Node {
    id: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    children: Option<Vec<Edge>>,
    zap_type: ZapType,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<NodeData>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde()]
pub struct Edge {
    id: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    condition: Option<Condition>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde()]
pub struct Condition {
    right: String,
    left: String,
    operator: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct DagNode {
    zap_type: ZapType,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<NodeData>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct DagEdge {
    #[serde(skip_serializing_if = "Option::is_none")]
    condition: Option<Condition>,
}

pub fn parse(dag_data: Vec<Node>) -> (daggy::Dag<DagNode, DagEdge>, Option<daggy::NodeIndex<u32>>) {
    let mut dag = daggy::Dag::<DagNode, DagEdge, u32>::new();

    let mut nodes = Vec::new();
    let mut nodes_map = HashMap::new();
    let mut root_node_index: Option<daggy::NodeIndex<u32>> = None;
    for node in &dag_data {
        let node_id =
            dag.add_node(DagNode { zap_type: node.zap_type.clone(), data: node.data.clone() });
        nodes.push(node_id);
        nodes_map.insert(node.id, node_id);

        if ZapType::Root == node.zap_type {
            root_node_index = Some(node_id);
        }
    }

    for node in &dag_data {
        if let Some(children) = &node.children {
            for child in children {
                let parent_index = nodes_map.get(&node.id).unwrap();
                let child_index = nodes_map.get(&child.id).unwrap();

                let result = dag.add_edge(
                    *parent_index,
                    *child_index,
                    DagEdge { condition: child.condition.clone() },
                );
                if let Err(e) = result {
                    panic!("Failed to add edge: {:?}", e);
                }
            }
        }
    }

    return (dag, root_node_index);
}

pub fn walk(
    dag: daggy::Dag<DagNode, DagEdge>,
    root_node_index: daggy::NodeIndex<u32>,
    vars: serde_json::Map<String, serde_json::Value>,
) {
    dag.children(root_node_index).iter(&dag).for_each(|child| {
        let child_node_index = child.1;
        let child_node = dag.node_weight(child_node_index).unwrap();
        let edge_index = child.0;
        let dag = dag.clone();

        if let Some(edge) = dag.edge_weight(edge_index) {
            if let Some(condition) = &edge.condition {
                let tmp_left: Value;
                let mut left_value: Option<&Value> = None;
                if condition.left.starts_with("$") {
                    left_value = vars.get(&condition.left);
                } else if condition.left.len() > 0 {
                    tmp_left = normalize_value(condition.left.clone());
                    left_value = Some(&tmp_left);
                }

                let tmp_right: Value;
                let mut right_value: Option<&Value> = None;
                if condition.right.starts_with("$") {
                    right_value = vars.get(&condition.right);
                } else if condition.right.len() > 0 {
                    tmp_right = normalize_value(condition.right.clone());
                    right_value = Some(&tmp_right);
                }

                if left_value.is_none() || right_value.is_none() {
                    panic!(
                        "Missing variable, left: {:?}, right: {:?}, child: {:?}",
                        left_value, right_value, child_node_index
                    );
                } else if let (Some(left), Some(right)) = (left_value, right_value) {
                    if !evaluate_condition(left, right, &condition.operator) {
                        return;
                    }
                }
            }
        }

        let mut new_vars = vars.clone();

        let mut done = false;
        match child_node.zap_type {
            ZapType::Arithmetic => {
                if let Some(data) = &child_node.data {
                    if data.left.is_none()
                        || data.right.is_none()
                        || data.operator.is_none()
                        || data.result.is_none()
                    {
                        panic!(
                            "Missing data, left: {:?}, right: {:?}, operator: {:?}, result: {:?}",
                            data.left, data.right, data.operator, data.result
                        );
                    }

                    let tmp_left: Value;
                    let mut left_value: Option<&Value> = None;
                    if data.left.clone().unwrap().starts_with("$") {
                        left_value = vars.get(&data.left.clone().unwrap());
                    } else if data.left.clone().unwrap().len() > 0 {
                        tmp_left = normalize_value(data.left.clone().unwrap());
                        left_value = Some(&tmp_left);
                    }

                    let tmp_right: Value;
                    let mut right_value: Option<&Value> = None;
                    if data.right.clone().unwrap().starts_with("$") {
                        right_value = vars.get(&data.right.clone().unwrap());
                    } else if data.right.clone().unwrap().len() > 0 {
                        tmp_right = normalize_value(data.right.clone().unwrap());
                        right_value = Some(&tmp_right);
                    }

                    if left_value.is_none() || right_value.is_none() {
                        panic!(
                            "Missing variable, left: {:?}, right: {:?}, child: {:?}",
                            left_value, right_value, child_node_index
                        );
                    }

                    let result_value = evaluate_arithmetic(
                        &left_value.unwrap(),
                        &right_value.unwrap(),
                        &data.operator.clone().unwrap(),
                    );

                    println!(
                        "{} = {} {} {}",
                        data.result.clone().unwrap(),
                        left_value.unwrap(),
                        data.operator.clone().unwrap(),
                        right_value.unwrap()
                    );
                    println!("Result: {:?}", result_value);
                    new_vars.insert(data.result.clone().unwrap(), result_value);

                    done = true;
                }
            },
            ZapType::Root => {
                done = true;
            },
            ZapType::Action => {
                if let Some(data) = &child_node.data {
                    if data.action_type.is_none()
                        || data.token_from_address.is_none()
                        || data.token_to_address.is_none()
                        || data.token_from_amount.is_none()
                    {
                        panic!("Missing data, action: {:?}", data);
                    }

                    match data.action_type.clone().unwrap() {
                        ActionType::SwapExactETHForTokens => {
                            let token_from_amount = data.token_from_amount.clone().unwrap();
                            let token_from_amount_value;
                            if token_from_amount.clone().starts_with("$") {
                                let tmp = vars.get(&data.right.clone().unwrap()).unwrap();
                                if tmp.is_u64() {
                                    token_from_amount_value = tmp.as_u64().unwrap();
                                } else {
                                    panic!("Invalid token_from_amount var: {:?}", tmp);
                                }
                            } else if token_from_amount.clone().len() > 0 {
                                let tmp = normalize_value(token_from_amount.clone());
                                if tmp.is_i64() {
                                    token_from_amount_value = tmp.as_u64().unwrap();
                                } else {
                                    panic!("Invalid token_from_amount: {:?}", tmp);
                                }
                            } else {
                                panic!("Invalid token_from_amount: {:?}", token_from_amount);
                            }

                            let token_from_address = data.token_from_address.clone().unwrap();
                            let token_to_address = data.token_to_address.clone().unwrap();

                            tokio::spawn(async move {
                                swap_exact_eth_for_tokens(
                                    token_from_address,
                                    token_to_address,
                                    token_from_amount_value,
                                )
                                .await;
                            });
                        },
                    }
                    done = true;
                }
            },
        }

        println!("Child: {:?}, parent: {:?}", child_node_index, root_node_index);

        if done {
            walk(dag, child_node_index, new_vars);
        } else {
            panic!("Faild in node: {:?}", child_node_index);
        }
    });
}

fn evaluate_condition(a: &serde_json::Value, b: &serde_json::Value, operator: &str) -> bool {
    if a.is_f64() && b.is_f64() {
        return check_condition_oprator(a.as_f64().unwrap(), b.as_f64().unwrap(), operator);
    } else if a.is_i64() && b.is_i64() {
        return check_condition_oprator(a.as_i64().unwrap(), b.as_i64().unwrap(), operator);
    } else if a.is_boolean() && b.is_boolean() {
        return check_condition_oprator(a.as_bool().unwrap(), b.as_bool().unwrap(), operator);
    } else if a.is_string() && b.is_string() {
        return check_condition_oprator(a.as_str().unwrap(), b.as_str().unwrap(), operator);
    }

    return false;
}

fn check_condition_oprator<T: PartialOrd>(a: T, b: T, operator: &str) -> bool {
    match operator {
        "==" => a == b,
        "!=" => a != b,
        ">" => a > b,
        ">=" => a >= b,
        "<" => a < b,
        "<=" => a <= b,
        _ => panic!("Unknown operator: {}", operator),
    }
}

fn normalize_value(s: String) -> Value {
    if s == "true" {
        return Value::Bool(true);
    } else if s == "false" {
        return Value::Bool(false);
    } else if let Ok(i) = s.parse::<i64>() {
        return Value::Number(Number::from(i));
    } else if let Ok(i) = s.parse::<u64>() {
        return Value::Number(Number::from(i));
    } else if let Ok(f) = s.parse::<f64>() {
        return Value::Number(Number::from_f64(f).unwrap());
    }

    return Value::String(s);
}

fn evaluate_arithmetic(
    a: &serde_json::Value,
    b: &serde_json::Value,
    operator: &str,
) -> serde_json::Value {
    if a.is_f64() && b.is_f64() {
        let tmp = evaluate_arithmetic_oprator(a.as_f64().unwrap(), b.as_f64().unwrap(), operator);
        return Value::Number(Number::from_f64(tmp).unwrap());
    } else if a.is_i64() && b.is_i64() {
        let tmp = evaluate_arithmetic_oprator(a.as_i64().unwrap(), b.as_i64().unwrap(), operator);
        return Value::Number(Number::from(tmp));
    } else if a.is_u64() && b.is_u64() {
        let tmp = evaluate_arithmetic_oprator(a.as_u64().unwrap(), b.as_u64().unwrap(), operator);
        return Value::Number(Number::from(tmp));
    }

    return Value::Null;
}

fn evaluate_arithmetic_oprator<
    T: Add<Output = T>
        + Rem<Output = T>
        + Sub<Output = T>
        + Mul<Output = T>
        + Div<Output = T>
        + Zero<Output = T>
        + PartialEq,
>(
    a: T,
    b: T,
    operator: &str,
) -> T {
    if operator == "/" && b == T::zero() {
        panic!("Division by zero");
    }

    match operator {
        "+" => a + b,
        "-" => a - b,
        "*" => a * b,
        "/" => a / b,
        "%" => a % b,
        _ => panic!("Unknown operator: {}", operator),
    }
}

pub async fn swap_exact_eth_for_tokens(from_address: String, to_address: String, from_amount: u64) {
    let provider_url = env::var("PROVIDER_URL").expect("PROVIDER_URL must be set");
    let http = web3::transports::Http::new(&provider_url).unwrap();
    let web3s = web3::Web3::new(http);

    let account_address = env::var("ACCOUNT_ADDRESS").expect("ACCOUNT_ADDRESS must be set");
    let account = H160::from_str(&account_address).expect("Invalid account address");

    let router02_address = env::var("ROUTER02_ADDRESS").expect("ROUTER02_ADDRESS must be set");
    let router02_addr = Address::from_str(&router02_address).expect("Invalid router02 address");
    let router02_contract =
        Contract::from_json(web3s.eth(), router02_addr, include_bytes!("./router02_abi.json"))
            .unwrap();

    let from_address = Address::from_str(&from_address).expect("Invalid from address");
    let to_address = Address::from_str(&to_address).expect("Invalid to address");

    let valid_timestamp = get_valid_timestamp(300000);

    let out_gas_estimate = router02_contract
        .estimate_gas(
            "swapExactETHForTokens",
            (
                U256::from(from_amount),
                vec![from_address, to_address],
                account,
                U256::from_dec_str(&valid_timestamp.to_string()).unwrap(),
            ),
            account,
            Options {
                gas: Some(500_000.into()),
                ..Default::default()
            },
        )
        .await
        .expect("Failed to estimate gas");

    let gas_price = web3s.eth().gas_price().await.expect("Failed to get gas price");

    let data = router02_contract
        .abi()
        .function("swapExactETHForTokens")
        .unwrap()
        .encode_input(
            &(
                U256::from(from_amount),
                vec![from_address, to_address],
                account,
                U256::from_dec_str(&valid_timestamp.to_string()).unwrap(),
            )
                .into_tokens(),
        )
        .expect("Failed to swapExactETHForTokens");

    let nonce = web3s.eth().transaction_count(account, None).await.unwrap() + valid_timestamp;
    let transact_obj = TransactionParameters {
        nonce: Some(nonce),
        to: Some(router02_addr),
        value: U256::exp10(18).checked_div(20.into()).unwrap(),
        gas_price: Some(gas_price),
        gas: out_gas_estimate,
        data: Bytes(data),
        ..Default::default()
    };

    let private_key_string = env::var("PRIVATE_KEY").expect("PRIVATE_KEY must be set");
    let private_key: SecretKey = private_key_string.parse().expect("Invalid private key");
    let signed_transaction = web3s
        .accounts()
        .sign_transaction(transact_obj, &private_key)
        .await
        .expect("Failed to sign transaction");

    let result = web3s
        .eth()
        .send_raw_transaction(signed_transaction.raw_transaction)
        .await
        .expect("Failed to send transaction");

    log::info!("Transaction successful with hash: {:?}", result);
}

fn get_valid_timestamp(future_millis: u128) -> u128 {
    let start = SystemTime::now();
    let since_epoch = start.duration_since(UNIX_EPOCH).unwrap();
    let time_millis = since_epoch.as_millis().checked_add(future_millis).unwrap();

    time_millis
}
