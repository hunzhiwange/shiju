import asyncio
async def run_tasks(tasks, max_concurrent):
    sem = asyncio.Semaphore(max_concurrent)
    async def execute(job):
        async with sem:
            await job()
    async with asyncio.TaskGroup() as group:
        for job in tasks:
            group.create_task(execute(job))
