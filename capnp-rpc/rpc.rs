/*
 * Copyright (c) 2014, David Renshaw (dwrenshaw@gmail.com)
 *
 * See the LICENSE file in the capnproto-rust root directory.
 */

use capnp::any::{AnyPointer};
use capnp::capability;
use capnp::capability::{RequestHook, ClientHook};
use capnp::common;
use capnp::message::{MessageReader, MessageBuilder, MallocMessageBuilder};
use capnp::serialize;
use std;
use rpc_capnp::{Message, Return, CapDescriptor};

type QuestionId = u32;
type AnswerId = QuestionId;
type ExportId = u32;
type ImportId = ExportId;

pub struct Question {
    is_awaiting_return : bool
}

pub struct Answer {
    result_exports : ~[ExportId]
}

pub struct Export;

pub struct Import;

pub struct ImportTable<T> {
    slots : ~[T]
}

pub struct ExportTable<T> {
    slots : ~[T]
}

pub struct RpcConnectionState {
    exports : ExportTable<Export>,
    questions : ExportTable<Question>,
    answers : ImportTable<Answer>,
    imports : ImportTable<Import>,
}

impl RpcConnectionState {
    pub fn new() -> RpcConnectionState {
        RpcConnectionState {
            exports : ExportTable { slots : ~[] },
            questions : ExportTable { slots : ~[] },
            answers : ImportTable { slots : ~[] },
            imports : ImportTable { slots : ~[] },
        }
    }
}

pub struct ImportClient {
    priv channel : std::comm::SharedChan<RpcEvent>,
    import_id : ImportId,
}

impl ClientHook for ImportClient {
    fn copy(&self) -> ~ClientHook {
        (box ImportClient {channel : self.channel.clone(),
                           import_id : self.import_id}) as ~ClientHook
    }

    fn new_call(&self, interface_id : u64, method_id : u16,
                _size_hint : Option<common::MessageSize>)
                -> capability::Request<AnyPointer::Builder, AnyPointer::Reader> {
        let hook = box RpcRequest { channel : self.channel.clone(),
                                    message : box MallocMessageBuilder::new_default() };
        capability::Request::new(hook as ~RequestHook)
    }
}

pub struct RpcRequest {
    priv channel : std::comm::SharedChan<RpcEvent>,
    priv message : ~MallocMessageBuilder
}

impl RequestHook for RpcRequest {
    fn message<'a>(&'a mut self) -> &'a mut MallocMessageBuilder {
        &mut *self.message
    }
    fn send(self) {
        let RpcRequest { channel, message } = self;
        channel.send(OutgoingMessage(message))
    }
}

pub enum RpcEvent {
    Nothing,
    IncomingMessage(~serialize::OwnedSpaceMessageReader),
    OutgoingMessage(~MallocMessageBuilder)
}

pub fn run_loop (port : std::comm::Port<RpcEvent>) {

    let mut connection_state = RpcConnectionState::new();

    loop {
        match port.recv() {
            IncomingMessage(message) => {
                let root = message.get_root::<Message::Reader>();

                match root.which() {
                    Some(Message::Return(ret)) => {
                        println!("got a return {}", ret.get_answer_id());
                        match ret.which() {
                            Some(Return::Results(payload)) => {
                                println!("with a payload");
                                let cap_table = payload.get_cap_table();
                                for ii in range(0, cap_table.size()) {
                                    match cap_table[ii].which() {
                                        Some(CapDescriptor::None(())) => {}
                                        Some(CapDescriptor::SenderHosted(id)) => {
                                            println!("sender hosted: {}", id);
                                        }
                                        _ => {}
                                    }
                                }
                            }
                            Some(Return::Exception(_)) => {
                                println!("exception");
                            }
                            _ => {}
                        }
                    }
                    Some(Message::Unimplemented(_)) => {
                        println!("unimplemented");
                    }
                    Some(Message::Abort(exc)) => {
                        println!("abort: {}", exc.get_reason());
                    }
                    None => { println!("Nothing there") }
                    _ => {println!("something else") }
                }
            }
            OutgoingMessage(mut m) => {
                let root = m.get_root::<Message::Builder>();
                match root.which() {
                    Some(Message::Which::Return(_)) => {}
                    Some(Message::Which::Call(_)) => {}
                    _ => {}
                }
                println!("outgoing message");
            }
            _ => {
                println!("got another event");
            }
        }
    }
}