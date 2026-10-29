import json, subprocess, sys, tempfile
from pathlib import Path
from selenium import webdriver
from selenium.webdriver.chrome.options import Options
from selenium.webdriver.chrome.service import Service
from selenium.webdriver.support.ui import WebDriverWait
from selenium.webdriver.support import expected_conditions as EC
from selenium.common.exceptions import TimeoutException
with tempfile.TemporaryDirectory() as d:
    p=Path(d)/'page.html';p.write_text(Path(sys.argv[1]).read_text())
    o=Options();o.binary_location='/usr/bin/chromium'
    for flag in ['--headless','--no-sandbox','--disable-dev-shm-usage','--disable-gpu']:o.add_argument(flag)
    driver=webdriver.Chrome(service=Service('/usr/bin/chromedriver'),options=o)
    alerts=[]
    try:
        driver.get(p.as_uri())
        try:
            WebDriverWait(driver,1).until(EC.alert_is_present())
            alerts.append(driver.switch_to.alert.text);driver.switch_to.alert.accept()
        except TimeoutException:pass
    finally:driver.quit()
    print(json.dumps(alerts))
