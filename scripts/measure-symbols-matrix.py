#!/usr/bin/env python3
"""Sequential actual-window matrix. Never compile/capture while cases are running."""
import subprocess,sys,argparse
parser=argparse.ArgumentParser();parser.add_argument('--binary',default='target/TekesUI Symbols Benchmark.app/Contents/MacOS/TekesUI-Symbols-Benchmark');parser.add_argument('--prefix',default='');parser.add_argument('--out',default='target/symbol-perf');args=parser.parse_args();binary=args.binary
cases=[('final-baseline-3',1,'steady',30,'0',False),('final-multi-24',8,'steady',60,'0',False),('final-parameters-24',8,'parameters',60,'0',False),('final-retarget-24',8,'retarget',60,'0',False),('final-combined-72',24,'combined',30,'0',False),('final-all-effects-42',14,'steady',20,'0',False),('diagnostic-static-24',8,'steady',30,'0',True),('diagnostic-gpu-timing-24',8,'steady',30,'1',False)]
for label,sources,mode,seconds,gpu,freeze in cases:
 label=args.prefix+label
 print('BEGIN '+label,flush=True)
 cmd=[sys.executable,'scripts/measure-symbols.py','--binary',binary,'--label',label,'--sources',str(sources),'--mode',mode,'--seconds',str(seconds),'--gpu-timing',gpu,'--out',args.out]
 if freeze:cmd+=['--freeze-raster']
 subprocess.run(cmd,check=True)
