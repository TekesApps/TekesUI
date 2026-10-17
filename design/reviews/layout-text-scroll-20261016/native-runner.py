import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys
import time

root = next(p for p in Path(__file__).resolve().parents if (p / 'Cargo.toml').is_file())
base = Path(__file__).resolve().parent
out = base / 'native-corrected'
out.mkdir(exist_ok=True)
binary = root / 'target/TekesUI Gallery.app/Contents/MacOS/TekesUI-Gallery'
digest = hashlib.sha256(binary.read_bytes()).hexdigest()
results = []
# External placement may fall back to the built-in screen. Fail before any
# native launch when the required display is unavailable.
displays = json.loads(subprocess.check_output(['/usr/sbin/system_profiler', 'SPDisplaysDataType', '-json'], text=True))
external = [screen for gpu in displays.get('SPDisplaysDataType', []) for screen in gpu.get('spdisplays_ndrvs', []) if screen.get('spdisplays_connection_type') != 'spdisplays_internal' and screen.get('spdisplays_online') == 'spdisplays_yes']
if not external:
    raise SystemExit('External display unavailable; native checks were not started.')
for mode in sys.argv[1:] or ['text', 'scroll-repair', 'reflow', 'workbench', 'workbench-extended']:
    captures = out / mode
    captures.mkdir(exist_ok=True)
    env = {key: os.environ[key] for key in ['HOME', 'USER', 'LOGNAME', 'TMPDIR', 'LANG'] if key in os.environ}
    env.update(PATH='/usr/bin:/bin:/usr/sbin:/sbin', TEKESUI_WINDOW_PLACEMENT='external', TEKES_LOG_EVENTS='1', TEKES_METAL_DISPLAY_LINK='0', TEKES_SELFTEST=mode, TEKES_SELFTEST_OUT=str(captures))
    start = time.monotonic()
    log = out / (mode + '.log')
    with log.open('w') as stream:
        process = subprocess.Popen([str(binary)], cwd=root, env=env, stdout=stream, stderr=subprocess.STDOUT, start_new_session=True)
        try:
            for _ in range(100):
                placement = log.read_text()
                if 'placement:' in placement or process.poll() is not None:
                    break
                time.sleep(0.01)
            if 'builtin=false' not in placement:
                process.terminate()
                process.wait(timeout=8)
                raise SystemExit('External placement was not confirmed; stopped owned process and aborted.')
            code = process.wait(timeout=100)
        except subprocess.TimeoutExpired:
            process.terminate()
            code = process.wait(timeout=8)
    data = log.read_text()
    row = dict(mode=mode, exit=code, seconds=round(time.monotonic()-start, 2), captures=len(list(captures.glob('*.png'))), selftest_fail='SELFTEST FAIL' in data, missing_menu_command='没有命令' in data, external='builtin=false' in data, binary_sha256=digest)
    row['capture_successes'] = data.count('Ok(ExitStatus(unix_wait_status(0)))')
    row['valid_run'] = code == 0 and not row['selftest_fail'] and not row['missing_menu_command'] and row['external'] and row['captures'] == row['capture_successes']
    results.append(row)
    (out / 'results.json').write_text(json.dumps(results, indent=2))
    print(json.dumps(row), flush=True)
