from pathlib import Path
import hashlib,subprocess,shutil,tempfile,struct,json
p=Path('target/gold7-alignment-pass2');result={};data=[]
with tempfile.TemporaryDirectory(prefix='verify-gallery-signature-',dir=p) as d:
 for name,src in [('app','target/TekesUI Gallery.app/Contents/MacOS/TekesUI-Gallery'),('cargo','target/presentation-repair-20261016/build/release/tekes-ui-gallery')]:
  dst=Path(d)/name;shutil.copy2(src,dst);subprocess.run(['codesign','--remove-signature',str(dst)],check=True,capture_output=True);b=bytearray(dst.read_bytes());assert struct.unpack_from('<I',b)[0]==0xfeedfacf
  offset=32;fields=[]
  for _ in range(struct.unpack_from('<I',b,16)[0]):
   cmd,size=struct.unpack_from('<II',b,offset)
   if cmd==0x19 and b[offset+8:offset+24].split(b'\0')[0]==b'__LINKEDIT':
    fields.append({'offset':offset+32,'field':'LC_SEGMENT_64.__LINKEDIT.vmsize','value':struct.unpack_from('<Q',b,offset+32)[0]});b[offset+32:offset+40]=bytes(8)
   offset+=size
  assert len(fields)==1
  result[name]={'original_sha256':hashlib.sha256(Path(src).read_bytes()).hexdigest(),'signature_removed_and_linkedit_vmsize_normalized_sha256':hashlib.sha256(b).hexdigest(),'normalized_metadata':fields};data.append(b)
assert data[0]==data[1], 'Executable bytes differ outside code signing metadata'
result['passed']=True;result['method']='Remove signatures only on disposable copies, then normalize only LC_SEGMENT_64 __LINKEDIT vmsize (codesign changes its allocation). All remaining executable bytes must be identical; original files unmodified.'
(p/'package-binary-comparison.json').write_text(json.dumps(result,indent=2)+'\n');print(json.dumps(result,indent=2))
