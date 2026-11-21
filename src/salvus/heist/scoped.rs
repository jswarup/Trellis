// scoped.rs ---------------------------------------------------------------------------------------
use crate::silo::{MutArr, USeg};

//-------------------------------------------------------------------------------------------------
/// Bounded static partitions of borrowed work. Threads join before return.
/// This executor has no job pool, global state, or work-stealing queues.
#[derive(Copy, Clone, Debug)]
pub struct ScopedExecutor
{
    _Workers: u32,
}

impl ScopedExecutor
{
    pub fn New(workers: u32) -> Self
    {
        Self { _Workers: workers.max(1) }
    }

    pub fn WorkerCount(&self, total: u32) -> u32
    {
        self._Workers.min(total)
    }

    /// Balanced nonempty partitions, with extra elements assigned to earlier workers.
    pub fn Partition(&self, total: u32, worker: u32) -> USeg
    {
        let count = self.WorkerCount(total);
        if worker >= count {
            return USeg::Empty();
        }
        let base = total / count;
        let remainder = total % count;
        let start = worker * base + worker.min(remainder);
        let len = base + u32::from(worker < remainder);
        USeg::WithLen(start, len)
    }

    pub fn ForEachRange<F>(&self, total: u32, action: F)
        where F: Fn(u32, u32, u32) + Sync
    {
        let count = self.WorkerCount(total);
        if count == 0 {
            return;
        }
        if count == 1 {
            action(0, 0, total);
            return;
        }
        std::thread::scope(|scope| {
            let action = &action;
            USeg::FromLen(count - 1).Traverse(|worker| {
                let partition = self.Partition(total, worker);
                scope.spawn(move || action(worker, partition.First(),
                                           partition.First() + partition.Size()));
            });
            let partition = self.Partition(total, count - 1);
            action(count - 1, partition.First(), total);
        });
    }

    /// Move disjoint mutable views to callbacks with their global starting offsets.
    pub fn ForEachMut<'a, T, F>(&self, values: MutArr<'a, T>, action: F)
        where T: Send,
              F: Fn(u32, u32, MutArr<'a, T>) + Sync
    {
        let count = self.WorkerCount(values.Len());
        if count != 0 {
            Self::RunMut(values, 0, 0, count, &action);
        }
    }

    fn RunMut<'a, T, F>(values: MutArr<'a, T>, worker: u32, start: u32, count: u32, action: &F)
        where T: Send,
              F: Fn(u32, u32, MutArr<'a, T>) + Sync
    {
        if count == 1 {
            action(worker, start, values);
            return;
        }
        let leftCount = count / 2;
        let leftLen = (values.Len() / count) * leftCount
            + (values.Len() % count).min(leftCount);
        let (left, right) = values.SplitAt(leftLen);
        std::thread::scope(|scope| {
            scope.spawn(move || Self::RunMut(left, worker, start, leftCount, action));
            Self::RunMut(right, worker + leftCount, start + leftLen, count - leftCount, action);
        });
    }
}

//-------------------------------------------------------------------------------------------------
