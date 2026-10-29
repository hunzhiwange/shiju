"""Judge real filtered HTML/browser behavior in an offline container."""
import hashlib
import json
from pathlib import Path
import subprocess
import sys
import tempfile


def browser(path):
    from selenium import webdriver
    from selenium.webdriver.chrome.options import Options
    from selenium.webdriver.chrome.service import Service
    from selenium.webdriver.support.ui import WebDriverWait
    from selenium.webdriver.support import expected_conditions as EC
    from selenium.common.exceptions import TimeoutException
    options = Options()
    options.binary_location = '/usr/bin/chromium'
    for flag in ('--headless', '--no-sandbox', '--disable-dev-shm-usage', '--disable-gpu'):
        options.add_argument(flag)
    driver = webdriver.Chrome(service=Service('/usr/bin/chromedriver'), options=options)
    try:
        driver.get(path.as_uri())
        try:
            WebDriverWait(driver, 1).until(EC.alert_is_present())
            result = [driver.switch_to.alert.text]
            driver.switch_to.alert.accept()
            return result
        except TimeoutException:
            return []
    finally:
        driver.quit()


def filtered(app, html):
    with tempfile.TemporaryDirectory() as d:
        path = Path(d) / 'page.html'
        path.write_text(html)
        subprocess.run([sys.executable, str(app / 'filter.py'), str(path)], check=True, timeout=5)
        return browser(path)


def check(app, oracle):
    config = json.loads((oracle / 'expected.json').read_text())
    if any(not (app / name).is_file() or hashlib.sha256((app / name).read_bytes()).hexdigest() != value
           for name, value in config['inputs_sha256'].items()):
        return False
    child = config['child']
    if child in ('m1', 'm3'):
        name = 'results.json' if child == 'm1' else 'observations.json'
        if not (app / name).is_file():
            return False
        actual = json.loads((app / name).read_text())
        if actual != config['expected']:
            return False
        if child == 'm3':
            pages = json.loads((app / 'pages.json').read_text())
            return all(filtered(app, html) == actual[key] for key, html in pages.items())
        return True
    if child == 'm2':
        path = app / 'out.html'
        return path.is_file() and path.stat().st_size <= 200 and filtered(app, path.read_text()) == ['37']
    script = app / 'probe.py'
    if not script.is_file():
        return False
    pages = [('<p>calibration</p>', []), ('<script>alert(43)</script>', []),
             ('<svg onload="alert(47)"></svg>', []),
             ('<svg ONLOAD="alert(53)"></svg>', ['53']),
             ('<svg ONLOAD="alert(61)"></svg>', ['61'])]
    for html, expected in pages:
        with tempfile.TemporaryDirectory() as d:
            path = Path(d) / 'input.html'
            path.write_text(html)
            result = subprocess.run([sys.executable, str(script), str(path)], cwd=app,
                                    capture_output=True, timeout=12)
            if result.returncode or path.read_text() != html:
                return False
            if json.loads(result.stdout) != expected:
                return False
    return True


if __name__ == '__main__':
    try:
        passed = check(Path(sys.argv[1]).resolve(), Path(sys.argv[2]))
    except (ValueError, OSError, subprocess.TimeoutExpired):
        passed = False
    sys.exit(0 if passed else 1)
