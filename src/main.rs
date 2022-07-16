use std::{
    sync::{Arc, Condvar, Mutex},
    thread::{self}, cell::RefCell,
};

pub trait Queue<E> {
    fn push(&mut self, v: E);
    fn pop(&mut self) -> E;
}

struct CyclicBuffer<E> {
    buf: Vec<E>,
    capacity: usize,
    size: usize,
    put_idx: usize,
    get_idx: usize,
}

impl<E: Default> CyclicBuffer<E> {
    fn new(capacity: usize) -> CyclicBuffer<E> {
        let mut buffer = Vec::with_capacity(capacity);
        buffer.resize_with(capacity, E::default);
        CyclicBuffer {
            buf: buffer,
            capacity: capacity,
            size: 0,
            put_idx: 0,
            get_idx: 0,
        }
    }
}

impl<E: Default> Queue<E> for CyclicBuffer<E> {
    fn push(&mut self, v: E) {
        self.buf.insert(self.put_idx, v);
        self.put_idx = (self.put_idx + 1) % self.capacity;
        self.size += 1;
    }

    fn pop(&mut self) -> E {
        let v_ref = self.buf.get_mut(self.get_idx).unwrap();
        self.get_idx = (self.get_idx + 1) % self.capacity;
        self.size -= 1;
        return std::mem::take(v_ref);
    }
}

impl<E: Clone> Clone for CyclicBuffer<E> {
    fn clone(&self) -> Self {
        Self {
            buf: self.buf.clone(),
            capacity: self.capacity,
            size: self.size,
            put_idx: self.put_idx,
            get_idx: self.get_idx,
        }
    }
}

struct BlockingQueue<E> {
    mutex: Mutex<RefCell<CyclicBuffer<E>>>,
    not_full: Condvar,
    not_empty: Condvar,
}

impl<E: Default> BlockingQueue<E> {
    fn new(capacity: usize) -> BlockingQueue<E> {
        BlockingQueue {
            mutex: Mutex::new(RefCell::new(CyclicBuffer::new(capacity))),
            not_full: Condvar::new(),
            not_empty: Condvar::new(),
        }
    }

    fn push(&self, v: E) {
        let mut buf = self.mutex.lock().unwrap();
        while buf.borrow().size == buf.borrow().capacity {
            buf = self.not_full.wait(buf).unwrap();
        }
        buf.borrow_mut().push(v);
        self.not_empty.notify_one();
    }

    fn pop(&self) -> E {
        let mut buf = self.mutex.lock().unwrap();
        while buf.borrow().size == 0 {
            buf = self.not_empty.wait(buf).unwrap();
        }
        let v = buf.borrow_mut().pop();
        self.not_full.notify_one();
        v
    }
}

fn main() {



    let bq: BlockingQueue<i32> = BlockingQueue::new(20);
    let shared_bq = Arc::new(bq);
    let mut prods = Vec::new();
    for i in 1..=100 {
        let sh_bq = shared_bq.clone();
        let th = thread::Builder::new()
            .name(format!("Producer thread №{}", i))
            .spawn(move || {
                for j in 1..=100 {
                    sh_bq.push(j);
                    println!("{} push {}", std::thread::current().name().unwrap(), j);
                }
            }).unwrap();
        prods.push(th);
    }

    let mut cons = Vec::new();
    for i in 1..=500 {
        let sh_bq = shared_bq.clone();
        let th = thread::Builder::new()
            .name(format!("Consumer thread №{}", i))
            .spawn(move || {
                for _ in 1..=20 {
                    let item = sh_bq.pop();
                    println!("{} pop {}", std::thread::current().name().unwrap(), item);
                }
            }).unwrap();
        cons.push(th);
    }
    for p in prods.into_iter() {
        let _ = p.join();
    }
    for c in cons.into_iter() {
        let _ = c.join();
    }
}
