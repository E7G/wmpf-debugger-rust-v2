use flate2::read::ZlibDecoder;
use flate2::write::DeflateEncoder;
use flate2::Compression;
use prost::Message;
use std::io::{Read, Write};

use crate::constants::{CompressAlgo, DebugMessageCategory};
use crate::proto::wa_remote_debug::*;

pub fn buffer_to_hex(buffer: &[u8]) -> String {
    buffer
        .iter()
        .map(|b| format!("{:02x}", b))
        .collect::<String>()
}

fn zlib_decompress(data: &[u8]) -> Vec<u8> {
    let mut decoder = ZlibDecoder::new(data);
    let mut decompressed = Vec::new();
    decoder.read_to_end(&mut decompressed).unwrap_or_default();
    decompressed
}

fn zlib_compress(data: &[u8]) -> Vec<u8> {
    let mut encoder = DeflateEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(data).unwrap_or_default();
    encoder.finish().unwrap_or_default()
}

pub struct WrapResult {
    pub buffer: Vec<u8>,
    pub original_size: u32,
}

#[allow(unused_assignments)]
pub fn wrap_debug_message_data(
    data: &serde_json::Value,
    category: &DebugMessageCategory,
    compress_algo: u32,
) -> WrapResult {
    let mut encoded: Option<Vec<u8>> = None;
    let mut original_size: u32 = 0;

    match category {
        DebugMessageCategory::CallInterface => {
            let msg = WaRemoteDebugCallInterface {
                obj_name: data["name"].as_str().unwrap_or("").to_string(),
                method_name: data["method"].as_str().unwrap_or("").to_string(),
                method_arg_list: data["args"]
                    .as_array()
                    .map(|a| {
                        a.iter()
                            .map(|v| v.as_str().unwrap_or("").to_string())
                            .collect()
                    })
                    .unwrap_or_default(),
                call_id: data["call_id"].as_str().unwrap_or("").to_string(),
            };
            let mut buf = Vec::new();
            msg.encode(&mut buf).unwrap();
            encoded = Some(buf);
        }
        DebugMessageCategory::EvaluateJavascriptResult => {
            let msg = WaRemoteDebugEvaluateJavascriptResult {
                ret: data["ret"].as_str().unwrap_or("").to_string(),
                evaluate_id: data["evaluate_id"].as_str().unwrap_or("").to_string(),
            };
            let mut buf = Vec::new();
            msg.encode(&mut buf).unwrap();
            encoded = Some(buf);
        }
        DebugMessageCategory::Ping => {
            let msg = WaRemoteDebugPing {
                ping_id: data["ping_id"].as_str().unwrap_or("").to_string(),
                payload: data["payload"]
                    .as_str()
                    .map(|s| s.as_bytes().to_vec())
                    .unwrap_or_default(),
            };
            let mut buf = Vec::new();
            msg.encode(&mut buf).unwrap();
            encoded = Some(buf);
        }
        DebugMessageCategory::Breakpoint => {
            let msg = WaRemoteDebugBreakpoint {
                is_hit: data["is_hit"].as_bool().unwrap_or(false),
            };
            let mut buf = Vec::new();
            msg.encode(&mut buf).unwrap();
            encoded = Some(buf);
        }
        DebugMessageCategory::DomOp => {
            let msg = WaRemoteDebugDomOp {
                params: data["params"].as_str().unwrap_or("").to_string(),
                webview_id: data["webview_id"].as_str().unwrap_or("").to_string(),
            };
            let mut buf = Vec::new();
            msg.encode(&mut buf).unwrap();
            encoded = Some(buf);
        }
        DebugMessageCategory::DomEvent => {
            let msg = WaRemoteDebugDomEvent {
                params: data["params"].as_str().unwrap_or("").to_string(),
                webview_id: data["webview_id"].as_str().unwrap_or("").to_string(),
            };
            let mut buf = Vec::new();
            msg.encode(&mut buf).unwrap();
            encoded = Some(buf);
        }
        DebugMessageCategory::ChromeDevtools => {
            let msg = WaRemoteDebugChromeDevtools {
                op_id: data["op_id"].as_u64().unwrap_or(0) as u32,
                payload: data["payload"].as_str().unwrap_or("").to_string(),
                jscontext_id: data["jscontext_id"].as_str().unwrap_or("").to_string(),
            };
            let mut buf = Vec::new();
            msg.encode(&mut buf).unwrap();
            encoded = Some(buf);
        }
        DebugMessageCategory::ConnectJsContext => {
            let msg = WaRemoteDebugConnectJsContext {
                jscontext_id: data["jscontext_id"].as_str().unwrap_or("").to_string(),
            };
            let mut buf = Vec::new();
            msg.encode(&mut buf).unwrap();
            encoded = Some(buf);
        }
        DebugMessageCategory::CustomMessage => {
            let msg = WaRemoteDebugCustomMessage {
                method: data["method"].as_str().unwrap_or("").to_string(),
                payload: data["payload"].as_str().unwrap_or("").to_string(),
                raw: data["raw"].as_str().unwrap_or("").to_string(),
            };
            let mut buf = Vec::new();
            msg.encode(&mut buf).unwrap();
            encoded = Some(buf);
        }
        _ => {
            return WrapResult {
                buffer: Vec::new(),
                original_size: 0,
            };
        }
    }

    let mut buf = match encoded {
        Some(b) => b,
        None => {
            return WrapResult {
                buffer: Vec::new(),
                original_size: 0,
            };
        }
    };

    if !buf.is_empty() && compress_algo & CompressAlgo::Zlib as u32 != 0 {
        original_size = buf.len() as u32;
        buf = zlib_compress(&buf);
    }

    WrapResult {
        buffer: buf,
        original_size,
    }
}

pub fn unwrap_debug_message_data(msg: &WaRemoteDebugDebugMessage) -> serde_json::Value {
    let mut data = msg.data.clone();
    let category_str = msg.category.clone();

    if !data.is_empty() && msg.compress_algo & CompressAlgo::Zlib as u32 != 0 {
        data = zlib_decompress(&data);
    }

    let parsed = match DebugMessageCategory::from_str(&category_str) {
        Some(cat) => decode_category_data(&cat, &data),
        None => {
            eprintln!("invalid debug object category");
            return serde_json::Value::Null;
        }
    };

    serde_json::json!({
        "seq": msg.seq,
        "delay": msg.after,
        "category": category_str,
        "data": parsed,
        "compress_algo": msg.compress_algo,
        "original_size": msg.original_size,
    })
}

fn decode_category_data(category: &DebugMessageCategory, data: &[u8]) -> serde_json::Value {
    match category {
        DebugMessageCategory::Breakpoint => {
            let msg = WaRemoteDebugBreakpoint::decode(data).unwrap_or_default();
            serde_json::json!({
                "is_hit": if msg.is_hit { 1 } else { 0 }
            })
        }
        DebugMessageCategory::CallInterface => {
            let msg = WaRemoteDebugCallInterface::decode(data).unwrap_or_default();
            serde_json::json!({
                "name": msg.obj_name,
                "method": msg.method_name,
                "args": msg.method_arg_list,
                "call_id": msg.call_id,
            })
        }
        DebugMessageCategory::CallInterfaceResult => {
            let msg = WaRemoteDebugCallInterfaceResult::decode(data).unwrap_or_default();
            serde_json::json!({
                "ret": msg.ret,
                "call_id": msg.call_id,
                "debug_info": msg.debug_info,
            })
        }
        DebugMessageCategory::EvaluateJavascript => {
            let msg = WaRemoteDebugEvaluateJavascript::decode(data).unwrap_or_default();
            serde_json::json!({
                "script": msg.script,
                "evaluate_id": msg.evaluate_id,
                "debug_info": msg.debug_info,
            })
        }
        DebugMessageCategory::EvaluateJavascriptResult => {
            let msg = WaRemoteDebugEvaluateJavascriptResult::decode(data).unwrap_or_default();
            serde_json::json!({
                "ret": msg.ret,
                "evaluate_id": msg.evaluate_id,
            })
        }
        DebugMessageCategory::Ping => {
            let msg = WaRemoteDebugPing::decode(data).unwrap_or_default();
            serde_json::json!({
                "ping_id": msg.ping_id,
                "payload": String::from_utf8_lossy(&msg.payload),
            })
        }
        DebugMessageCategory::Pong => {
            let msg = WaRemoteDebugPong::decode(data).unwrap_or_default();
            serde_json::json!({
                "ping_id": msg.ping_id,
                "network_type": msg.network_type,
                "payload": String::from_utf8_lossy(&msg.payload),
            })
        }
        DebugMessageCategory::SetupContext => {
            let msg = WaRemoteDebugSetupContext::decode(data).unwrap_or_default();
            let ri = msg.register_interface.unwrap_or_default();
            let di = msg.device_info.unwrap_or_default();
            serde_json::json!({
                "register_interface": {
                    "obj_name": ri.obj_name,
                    "obj_methods": ri.obj_method_list.into_iter().map(|m| {
                        serde_json::json!({
                            "method_name": m.method_name,
                            "method_args": m.method_arg_list,
                        })
                    }).collect::<Vec<_>>(),
                },
                "configure_js": msg.configure_js,
                "public_js_md5": msg.public_js_md5,
                "three_js_md5": msg.three_js_md5,
                "device_info": {
                    "device_name": di.device_name,
                    "device_model": di.device_model,
                    "os": di.system_version,
                    "wechat_version": di.wechat_version,
                    "pixel_ratio": di.pixel_ratio,
                    "screen_width": di.screen_width,
                    "publib": di.publib_version,
                    "user_agent": di.user_agent,
                },
                "support_compress_algo": msg.support_compress_algo,
            })
        }
        DebugMessageCategory::DomOp => {
            let msg = WaRemoteDebugDomOp::decode(data).unwrap_or_default();
            serde_json::json!({
                "params": msg.params,
                "webview_id": msg.webview_id,
            })
        }
        DebugMessageCategory::DomEvent => {
            let msg = WaRemoteDebugDomEvent::decode(data).unwrap_or_default();
            serde_json::json!({
                "params": msg.params,
                "webview_id": msg.webview_id,
            })
        }
        DebugMessageCategory::NetworkDebugAPI => {
            let msg = WaRemoteDebugNetworkDebugApi::decode(data).unwrap_or_default();
            serde_json::json!({
                "api_name": msg.api_name,
                "task_id": msg.task_id,
                "request_headers": msg.request_headers,
                "timestamp": msg.timestamp,
            })
        }
        DebugMessageCategory::ChromeDevtools => {
            let msg = WaRemoteDebugChromeDevtools::decode(data).unwrap_or_default();
            serde_json::json!({
                "op_id": msg.op_id,
                "payload": msg.payload,
                "jscontext_id": msg.jscontext_id,
            })
        }
        DebugMessageCategory::ChromeDevtoolsResult => {
            let msg = WaRemoteDebugChromeDevtoolsResult::decode(data).unwrap_or_default();
            serde_json::json!({
                "op_id": msg.op_id,
                "payload": msg.payload,
                "jscontext_id": msg.jscontext_id,
            })
        }
        DebugMessageCategory::AddJsContext => {
            let msg = WaRemoteDebugAddJsContext::decode(data).unwrap_or_default();
            serde_json::json!({
                "jscontext_id": msg.jscontext_id,
                "jscontext_name": msg.jscontext_name,
            })
        }
        DebugMessageCategory::RemoveJsContext => {
            let msg = WaRemoteDebugRemoveJsContext::decode(data).unwrap_or_default();
            serde_json::json!({
                "jscontext_id": msg.jscontext_id,
            })
        }
        DebugMessageCategory::ConnectJsContext => {
            let msg = WaRemoteDebugConnectJsContext::decode(data).unwrap_or_default();
            serde_json::json!({
                "jscontext_id": msg.jscontext_id,
            })
        }
        DebugMessageCategory::CustomMessage => {
            let msg = WaRemoteDebugCustomMessage::decode(data).unwrap_or_default();
            serde_json::json!({
                "method": msg.method,
                "payload": msg.payload,
                "raw": msg.raw,
            })
        }
        _ => serde_json::Value::Null,
    }
}

#[allow(dead_code)]
pub fn unwrap_proto_to_data_format(data: &[u8]) -> serde_json::Value {
    let msg = match WaRemoteDebugDataFormat::decode(data) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("error decoding DataFormat: {}", e);
            return serde_json::json!({ "error": e.to_string() });
        }
    };

    let cmd_name = crate::constants::Constants::response_type_name(msg.cmd);
    let mut parsed = serde_json::Value::Null;

    match msg.cmd {
        1000 => {
            // MessageNotify
            let notify = WaRemoteDebugMessageNotify::decode(&*msg.data).unwrap_or_default();
            let mut messages = Vec::new();
            for dm in &notify.debug_message_list {
                messages.push(unwrap_debug_message_data(dm));
            }
            parsed = serde_json::json!({ "debug_message": messages });
        }
        1006 => {
            // MessageNotifyParallelly
            let notify = WaRemoteDebugMessageNotify::decode(&*msg.data).unwrap_or_default();
            let mut messages = Vec::new();
            for dm in &notify.debug_message_list {
                messages.push(unwrap_debug_message_data(dm));
            }
            parsed = serde_json::json!({ "debug_message": messages });
        }
        2000 => {
            // SendDebugMessage
            let resp = WaRemoteDebugSendDebugMessageResp::decode(&*msg.data).unwrap_or_default();
            parsed = serde_json::json!({
                "base_response": resp.base_response.map(|r| serde_json::json!({
                    "errcode": r.errcode,
                    "errmsg": r.errmsg,
                })),
                "send_ack": resp.send_ack,
            });
        }
        2002 => {
            // Login
            let resp = WaRemoteDebugDevLoginResp::decode(&*msg.data).unwrap_or_default();
            let ri = resp.room_info.unwrap_or_default();
            parsed = serde_json::json!({
                "base_response": resp.base_response.map(|r| serde_json::json!({
                    "errcode": r.errcode,
                    "errmsg": r.errmsg,
                })),
                "room_info": {
                    "join_room": ri.join_room,
                    "original_md5": ri.original_md5,
                    "room_status": ri.room_status,
                    "wx_conn_status": ri.wx_conn_status,
                    "dev_conn_status": ri.dev_conn_status,
                    "room_id": ri.room_id,
                },
            });
        }
        2003 => {
            // JoinRoom
            let resp = WaRemoteDebugDevJoinRoomResp::decode(&*msg.data).unwrap_or_default();
            parsed = serde_json::json!({
                "base_response": resp.base_response.map(|r| serde_json::json!({
                    "errcode": r.errcode,
                    "errmsg": r.errmsg,
                })),
            });
        }
        2001 => {
            // Heartbeat
            let resp = WaRemoteDebugDevHeartBeatResp::decode(&*msg.data).unwrap_or_default();
            parsed = serde_json::json!({
                "base_response": resp.base_response.map(|r| serde_json::json!({
                    "errcode": r.errcode,
                    "errmsg": r.errmsg,
                })),
            });
        }
        2004 => {
            // QuitRoom
            let resp = WaRemoteDebugDevQuitRoomResp::decode(&*msg.data).unwrap_or_default();
            parsed = serde_json::json!({
                "base_response": resp.base_response.map(|r| serde_json::json!({
                    "errcode": r.errcode,
                    "errmsg": r.errmsg,
                })),
            });
        }
        2006 => {
            // SendDebugMessageParallelly
            let resp = WaRemoteDebugNewSendDebugMessageResp::decode(&*msg.data).unwrap_or_default();
            parsed = serde_json::json!({
                "base_response": resp.base_response.map(|r| serde_json::json!({
                    "errcode": r.errcode,
                    "errmsg": r.errmsg,
                })),
                "max_ack": resp.max_ack,
                "min_ack": resp.min_ack,
            });
        }
        2005 => {
            // SyncMessage
            let resp = WaRemoteDebugDevSyncMessageResp::decode(&*msg.data).unwrap_or_default();
            let mut messages = Vec::new();
            for dm in &resp.debug_message_list {
                messages.push(unwrap_debug_message_data(dm));
            }
            parsed = serde_json::json!({
                "debug_message": messages,
                "send_ack": resp.send_ack,
                "base_response": resp.base_response.map(|r| serde_json::json!({
                    "errcode": r.errcode,
                    "errmsg": r.errmsg,
                })),
            });
        }
        3001..=3003 => {
            // EventNotify
            let resp = WaRemoteDebugEventNotify::decode(&*msg.data).unwrap_or_default();
            parsed = serde_json::json!({
                "base_response": resp.base_response.map(|r| serde_json::json!({
                    "errcode": r.errcode,
                    "errmsg": r.errmsg,
                })),
            });
        }
        _ => {
            eprintln!("error receive invalid cmd {}", msg.cmd);
        }
    }

    serde_json::json!({
        "cmd": msg.cmd,
        "uuid": msg.uuid,
        "data": parsed,
        "_comment": cmd_name,
    })
}

#[allow(dead_code)]
pub fn wrap_outgoing_to_proto(_data: &serde_json::Value, request_type: i32, uuid: &str) -> Vec<u8> {
    let cmd = request_type_to_cmd(request_type);
    let mut inner_data: Vec<u8> = Vec::new();

    match request_type {
        1006 => {
            // MessageNotifyParallelly -> response
            let msg = WaRemoteDebugSendDebugMessageResp {
                base_response: Some(WaRemoteDebugBaseResp {
                    errcode: 0,
                    errmsg: String::new(),
                }),
                ..Default::default()
            };
            msg.encode(&mut inner_data).unwrap();
        }
        _ => {
            eprintln!(
                "error wrapping outgoing object, invalid type {}",
                request_type
            );
            return Vec::new();
        }
    }

    let data_format = WaRemoteDebugDataFormat {
        cmd: cmd as u32,
        uuid: uuid.to_string(),
        data: inner_data,
    };
    let mut buf = Vec::new();
    data_format.encode(&mut buf).unwrap();
    buf
}

#[allow(dead_code)]
fn request_type_to_cmd(request_type: i32) -> i32 {
    match request_type {
        2001 => 1001,
        2002 => 1002,
        3001 => 3001,
        3002 => 3002,
        3003 => 3003,
        2003 => 1003,
        2000 => 1000,
        2006 => 1006,
        2004 => 1004,
        1000 => 2000,
        1006 => 2006,
        2005 => 1005,
        _ => -1,
    }
}
