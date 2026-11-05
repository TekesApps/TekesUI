#!/usr/bin/env python3
"""Opt-in original PID-key source; requires an existing exact foreground fixture.
No launch, activation, input-source switch, permission request, retry or cleanup kill.
Run only inside the coordinator's existing native/input occupancy guard.
MacInput implementation and read-only foreground check derive from AgentKit8734185.
"""
import argparse
import ctypes as c
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import time

class MacInput:
    def __init__(self):
        self.cf = c.CDLL('/System/Library/Frameworks/CoreFoundation.framework/CoreFoundation')
        self.cg = c.CDLL('/System/Library/Frameworks/CoreGraphics.framework/CoreGraphics')
        self.tis = c.CDLL('/System/Library/Frameworks/Carbon.framework/Carbon')
        self.cf.CFRelease.argtypes = [c.c_void_p]
        self.cf.CFStringGetCString.argtypes = [c.c_void_p, c.c_char_p, c.c_long, c.c_uint32]
        self.cf.CFStringGetCString.restype = c.c_bool
        self.tis.TISCopyCurrentKeyboardInputSource.restype = c.c_void_p
        self.tis.TISGetInputSourceProperty.argtypes = [c.c_void_p, c.c_void_p]
        self.tis.TISGetInputSourceProperty.restype = c.c_void_p
        self.cg.CGPreflightPostEventAccess.restype = c.c_bool
        self.cg.CGEventCreateKeyboardEvent.argtypes = [c.c_void_p, c.c_uint16, c.c_bool]
        self.cg.CGEventCreateKeyboardEvent.restype = c.c_void_p
        self.cg.CGEventPostToPid.argtypes = [c.c_int, c.c_void_p]
        self.cg.CGEventSetFlags.argtypes = [c.c_void_p, c.c_uint64]

    def source(self):
        source = self.tis.TISCopyCurrentKeyboardInputSource()
        if not source:
            raise RuntimeError('current input source unavailable')
        try:
            key = c.c_void_p.in_dll(self.tis, 'kTISPropertyInputSourceID').value
            value = self.tis.TISGetInputSourceProperty(source, key)
            buf = c.create_string_buffer(1024)
            if not value or not self.cf.CFStringGetCString(value, buf, len(buf), 0x08000100):
                raise RuntimeError('input-source ID unavailable')
            return buf.value.decode('utf-8')
        finally:
            self.cf.CFRelease(source)

    def key(self, pid, code):
        for down in (True, False):
            event = self.cg.CGEventCreateKeyboardEvent(None, code, down)
            if not event:
                raise RuntimeError('keyboard event allocation failed')
            try:
                self.cg.CGEventSetFlags(event, 0)
                self.cg.CGEventPostToPid(pid, event)
            finally:
                self.cf.CFRelease(event)
            time.sleep(0.03)


def frontmost_identity():
    asn = subprocess.check_output(['/usr/bin/lsappinfo', 'front'], text=True, timeout=2).strip()
    if not re.fullmatch(r'ASN:0x[0-9a-fA-F]+-0x[0-9a-fA-F]+:', asn):
        raise ValueError('frontmost ASN unavailable')
    text = subprocess.check_output(['/usr/bin/lsappinfo', 'info', '-only', 'pid,bundleid,name', asn], text=True, timeout=2)
    pid = re.search(r'^"pid"=(\d+)$', text, re.MULTILINE)
    bundle = re.search(r'^"CFBundleIdentifier"="([^"\n]*)"$', text, re.MULTILINE)
    if pid is None:
        raise ValueError('frontmost PID unavailable')
    return {'asn': asn, 'pid': int(pid[1]), 'bundleId': bundle[1] if bundle else None}

def executable_for_pid(pid):
    library = c.CDLL('/usr/lib/libproc.dylib')
    library.proc_pidpath.argtypes = [c.c_int, c.c_void_p, c.c_uint32]
    library.proc_pidpath.restype = c.c_int
    buffer = c.create_string_buffer(4096)
    if library.proc_pidpath(pid, buffer, len(buffer)) <= 0:
        raise RuntimeError('owned demo executable is unavailable')
    return Path(buffer.value.decode('utf-8')).resolve(strict=True)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--pid', required=True, type=int)
    parser.add_argument('--program', required=True, type=Path)
    parser.add_argument('--log', required=True, type=Path)
    parser.add_argument('--receipt', required=True, type=Path)
    parser.add_argument('--timeout', type=float, default=25)
    args = parser.parse_args()
    program = args.program.resolve(strict=True)
    mac = MacInput()
    expected_source = 'com.apple.inputmethod.SCIM.ITABC'
    def verify():
        identity = frontmost_identity()
        if identity['pid'] != args.pid or not os.path.samefile(executable_for_pid(args.pid), program):
            raise RuntimeError('exact fixture is not foreground; refusing input')
        if mac.source() != expected_source:
            raise RuntimeError('Simplified Pinyin is not current; never switch it in this driver')
        if not mac.cg.CGPreflightPostEventAccess():
            raise RuntimeError('event-post permission unavailable; no permission request')
        return identity
    # Fresh receipt preserves even an early guard failure. No events before verification.
    with args.receipt.open('x') as receipt:
        def record(value):
            value = dict(value, recorded_monotonic_ns=time.monotonic_ns())
            receipt.write(json.dumps(value, ensure_ascii=False) + '\n'); receipt.flush()
        record({'method': 'CGEventPostToPid', 'pid': args.pid,
                'program_sha256': hashlib.sha256(program.read_bytes()).hexdigest(),
                'driver_sha256': hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
                'keys': [45, 34, 4, 0, 31, 49], 'down_up_spacing_seconds': 0.03,
                'physical_keyboard': False, 'unicode_insertion': False})
        verify()
        deadline = time.monotonic() + args.timeout
        typed = False
        while time.monotonic() < deadline:
            rows = []
            if args.log.exists():
                # Only complete lines; a partially written UTF-8 record is not parsed yet.
                content = args.log.read_bytes()
                for line in content.split(b'\n')[:-1]:
                    if line.startswith(b'UNICODE41 '): rows.append(json.loads(line[len(b'UNICODE41 '):]))
            ready = [r for r in rows if r['kind'] == 'ready']
            if ready and not typed:
                if len(ready) != 1 or ready[0]['pid'] != args.pid or ready[0]['declared_source'] != 'pid-hardware-key' or ready[0]['case'] != 'pinyin-first' or not ready[0]['active'] or not ready[0]['focused'] or ready[0]['value'] or ready[0]['marked'] is not None:
                    raise RuntimeError('not one fresh active/focused empty Pinyin fixture')
                for code in [45, 34, 4, 0, 31]:
                    record({'stage': 'TYPE', 'code': code, 'foreground': verify(), 'source': mac.source()})
                    mac.key(args.pid, code)
                    record({'stage': 'KEY_PAIR_RETURNED', 'code': code})
                typed = True
            ends = [r for r in rows if r['kind'] == 'key_context' and r['detail']['phase'] == 'end']
            if typed and len(ends) >= 5:
                if len(ends) != 5:
                    raise RuntimeError('unexpected external keys; retaining failed input evidence')
                record({'stage': 'COMMIT', 'code': 49, 'foreground': verify(), 'source': mac.source(),
                        'trigger': 'five original keyDown spans finished; no warmup or retry'})
                mac.key(args.pid, 49)
                record({'stage': 'KEY_PAIR_RETURNED', 'code': 49})
                record({'source_after': mac.source(), 'injection_completed': True,
                        'result_not_yet_verified': True})
                return
            time.sleep(0.01)
        raise RuntimeError('input stage timed out; retain first failure, do not relaunch or retry')


if __name__ == '__main__':
    main()
