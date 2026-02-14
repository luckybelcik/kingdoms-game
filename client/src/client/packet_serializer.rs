use std::sync::mpsc::{Receiver, Sender};

use engine_net::{client_packet::ClientPacket, server_packet::ServerPacket};

type SerializingJob = PacketOrBytes;
type FinishedTask = PacketOrBytes;

pub struct PacketSerializer {
    job_sender: Sender<SerializingJob>,
    task_receiver: Receiver<FinishedTask>,
}

impl PacketSerializer {
    pub fn create() -> Self {
        let (job_sender, job_receiver) = std::sync::mpsc::channel::<SerializingJob>();
        let (task_sender, task_receiver) = std::sync::mpsc::channel::<FinishedTask>();

        std::thread::Builder::new()
            .name("ClientSerializingThread".to_string())
            .spawn(move || {
                for job in job_receiver {
                    let sender_clone = task_sender.clone();

                    rayon::spawn(move || match job {
                        PacketOrBytes::Client(packet) => {
                            match bincode::serialize::<ClientPacket>(&packet) {
                                Ok(p) => sender_clone
                                    .send(PacketOrBytes::ClientBytes(p))
                                    .expect("Failed to send client bytes"),
                                Err(e) => {
                                    eprintln!("Client packet serialization error: {:?}.", e)
                                }
                            }
                        }
                        PacketOrBytes::Server(packet) => {
                            match bincode::serialize::<ServerPacket>(&packet) {
                                Ok(p) => sender_clone
                                    .send(PacketOrBytes::ServerBytes(p))
                                    .expect("Failed to send server bytes"),
                                Err(e) => {
                                    eprintln!("Server packet serialization error: {:?}.", e)
                                }
                            }
                        }
                        PacketOrBytes::ClientBytes(bytes) => {
                            match bincode::deserialize::<ClientPacket>(&bytes) {
                                Ok(p) => sender_clone
                                    .send(PacketOrBytes::Client(p))
                                    .expect("Failed to receive client packet"),
                                Err(e) => {
                                    eprintln!(
                                        "Client packet deserialization error: {:?}. Bytes: {:?}",
                                        e, bytes
                                    )
                                }
                            }
                        }
                        PacketOrBytes::ServerBytes(bytes) => {
                            match bincode::deserialize::<ServerPacket>(&bytes) {
                                Ok(p) => sender_clone
                                    .send(PacketOrBytes::Server(p))
                                    .expect("Failed to receive server packet"),
                                Err(e) => {
                                    eprintln!(
                                        "Server packet deserialization error: {:?}. Bytes: {:?}",
                                        e, bytes
                                    )
                                }
                            }
                        }
                    });
                }
            })
            .unwrap();

        return PacketSerializer {
            job_sender,
            task_receiver,
        };
    }

    pub fn deserialize_server_packet_bytes(&self, bytes: Vec<u8>) {
        self.job_sender
            .send(PacketOrBytes::ServerBytes(bytes))
            .unwrap();
    }

    pub fn receive_finished_tasks(&self) -> Vec<ServerPacket> {
        let mut finished_tasks = Vec::new();

        while let Ok(task) = self.task_receiver.try_recv() {
            if let PacketOrBytes::Server(server_packet) = task {
                finished_tasks.push(server_packet);
            } else {
                panic!("guys dont do this");
            }
        }

        finished_tasks
    }
}

enum PacketOrBytes {
    Client(ClientPacket),
    Server(ServerPacket),
    ClientBytes(Vec<u8>),
    ServerBytes(Vec<u8>),
}
