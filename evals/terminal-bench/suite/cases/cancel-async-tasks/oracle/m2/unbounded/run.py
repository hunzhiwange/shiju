import asyncio
async def run_tasks(tasks, max_concurrent):
    await asyncio.gather(*(job() for job in tasks))
