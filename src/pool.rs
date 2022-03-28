use std::thread::{self, Builder};
use std::result::Result;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use ::flume::{Sender, Receiver, SendError, TrySendError, RecvError, TryRecvError};
use crate::task::Task;

struct TaskWoker<T> {
     recver:Receiver<T>,
     inner: Arc<Inner<T>>,
}

pub struct TaskSender<T> {
    sender: Sender<T>,
    inner: Arc<Inner<T>>,
}

struct Inner<T>{
    pool_size:usize,
    max_task_count: usize,
    current_worker_count: Mutex<usize>,
    recver: Receiver<T>,
}

pub struct ThreadPool<T> {
    inner: Arc<Inner<T>>,
}

impl<T: Task> Inner<T> {
    fn new (pool_size: usize, max_task_count: usize, rx: Receiver<T>) -> Self {
        Inner{
            pool_size,
            max_task_count,
            current_worker_count: Mutex::new(0),
            recver:rx,
        }
    }

    fn add_worker(&self, task: Option<T>, arc: &Arc<Inner<T>>) -> Result<(), Option<T>> {
        if self.has_capacity() {
            let tasked = TaskWoker{
                recver: self.recver.clone(),
                inner: arc.clone(),
            };
            tasked.spawn(task);
            self.inc_worker_count();
            Ok(())
        }
        else{
            Err(task)
        }
    }

    fn inc_worker_count(&self) {
        let mut lock = self.current_worker_count.lock().unwrap();
        *lock += 1;
        drop(lock);
    }

    fn dec_worker_count(&self) {
        let mut lock = self.current_worker_count.lock().unwrap();
        *lock -= 1;
        drop(lock);
    }

    fn has_capacity(&self) -> bool {
        let lock = self.current_worker_count.lock().unwrap();
        *lock < self.pool_size
    }

}

impl<T: Task> ThreadPool<T> {
    pub fn new(pool_size: usize, max_task_count: usize) -> (TaskSender<T>, ThreadPool<T>) {
        let (tx, rx) = flume::bounded(max_task_count);
        let inner = Arc::new(
            Inner{
                pool_size,
                max_task_count,
                current_worker_count: Mutex::new(0),
                recver: rx,
            }
        );

        let sender = TaskSender {
            sender: tx,
            inner: inner.clone(),
        };

        let pool = ThreadPool{
            inner:inner,
        };

        (sender, pool)
    }

}

impl<T: Task> TaskSender<T> {
    pub fn add_task(&self, task: T) ->Result<(), SendError<T>>{
        match self.try_send(task) {
            Ok(_) => Ok(()),
            Err(TrySendError::Disconnected(task)) => Err(SendError(task)),
            Err(TrySendError::Full(task)) => {
                self.sender.send(task)
            }
        }
    }

    fn try_send(&self, task: T) -> Result<(), TrySendError<T>>{
        match self.sender.try_send(task) {
            Ok(_) => {
                if self.inner.has_capacity() {
                    self.inner.add_worker(None, &self.inner);
                }
                Ok(())
            }
            Err(TrySendError::Disconnected(task)) => {
                return Err(TrySendError::Disconnected(task));
            }
            Err(TrySendError::Full(task)) => {
                match self.inner.add_worker(Some(task), &self.inner) {
                    Ok(_) => return Ok(()),
                    Err(task) =>  return Err(TrySendError::Full(task.unwrap())),
                }
            }
        }
    }


}

impl<T: Task> TaskWoker<T> {
    pub fn spawn(self, task: Option<T>) {
        let mut worker_thread = thread::Builder::new();
        worker_thread.spawn(move || self.run(task));
    }

    pub fn run(mut self, mut inittask: Option<T>) {
        while let Some(task) = self.next_task(inittask.take()){
            task.run();
        }
    }

    fn next_task(&mut self, mut task: Option<T>) -> Option<T> {
        loop {
            if task.is_some(){
                break;
            }
            match self.recv_task() {
                Ok(t) => task = Some(t),
                Err(RecvError) => {
                    self.inner.dec_worker_count();
                    return None;
                }
            }
        }
        task
    }

    fn recv_task(&self) -> Result<T, RecvError> {
        match self.recver.try_recv() {
            Ok(task) => return Ok(task),
            Err(TryRecvError::Disconnected) => {
                return Err(RecvError::Disconnected);
            }
            Err(TryRecvError::Empty) => {
                match self.recver.recv() {
                    Ok(task) => return Ok(task),
                    Err(RecvError) => return Err(RecvError::Disconnected),
                }
            }
        }
    }
}




