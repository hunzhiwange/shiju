async def run_tasks(tasks, max_concurrent):
    for job in tasks:
        await job()
