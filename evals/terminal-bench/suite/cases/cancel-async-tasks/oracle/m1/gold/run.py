def validate_tasks(tasks, max_concurrent):
    if type(tasks) is not list or type(max_concurrent) is not int:
        raise TypeError()
    if max_concurrent < 1:
        raise ValueError()
    if any(not callable(job) for job in tasks):
        raise TypeError()
    return None
