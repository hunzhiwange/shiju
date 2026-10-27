import asyncio
async def run_tasks(tasks, max_concurrent):
    children=[asyncio.create_task(job()) for job in tasks[:max_concurrent]]
    try:
        await asyncio.wait(children)
    finally:
        for child in children:
            child.cancel()
