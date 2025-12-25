use std::sync::mpsc::{Receiver, Sender};

use crate::shared::communication::{client_packet::ClientPacket, server_packet::ServerPacket};

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
                            let serialized = bincode::serialize(&packet).unwrap();
                            sender_clone
                                .send(PacketOrBytes::ClientBytes(serialized))
                                .expect("Failed to send client bytes");
                        }
                        PacketOrBytes::Server(packet) => {
                            let serialized = bincode::serialize(&packet).unwrap();
                            sender_clone
                                .send(PacketOrBytes::ServerBytes(serialized))
                                .expect("Failed to send server bytes");
                        }
                        PacketOrBytes::ClientBytes(bytes) => {
                            let deserialized = bincode::deserialize(&bytes).unwrap();
                            sender_clone
                                .send(PacketOrBytes::Client(deserialized))
                                .expect("Failed to send client packet");
                        }
                        PacketOrBytes::ServerBytes(bytes) => {
                            let deserialized = bincode::deserialize(&bytes).unwrap();
                            sender_clone
                                .send(PacketOrBytes::Server(deserialized))
                                .expect("Failed to send server packet");
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
