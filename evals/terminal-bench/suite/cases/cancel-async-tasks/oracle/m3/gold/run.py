import asyncio

async def run_tasks(tasks, max_concurrent):
    jobs = iter(tasks)
    pending = set()
    def fill():
        while len(pending) < max_concurrent:
            job = next(jobs, None)
            if job is None:
                break
            pending.add(asyncio.create_task(job()))
    try:
        fill()
        while pending:
            done, rest = await asyncio.wait(pending, return_when=asyncio.FIRST_COMPLETED)
            for task in done:
                task.result()
            pending = rest
            fill()
    finally:
        for task in pending:
            task.cancel()
        await asyncio.gather(*pending, return_exceptions=True)
