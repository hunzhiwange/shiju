import asyncio
async def run_tasks(tasks, max_concurrent):
    sem=asyncio.Semaphore(max_concurrent)
    async def one(job):
        async with sem:
            await job()
    await asyncio.gather(*(one(job) for job in tasks))
