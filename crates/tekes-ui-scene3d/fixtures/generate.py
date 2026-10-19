"""Deterministic MIT test assets authored for TekesUI; no downloaded artwork."""
import json, struct, zlib, base64
from pathlib import Path
root = Path(__file__).parent
blob = bytearray()
views, accessors = [], []
def add(values, kind, dims, component=5126, **extra):
    while len(blob)%4: blob.append(0)
    data = struct.pack('<'+kind*len(values), *values)
    views.append(dict(buffer=0, byteOffset=len(blob), byteLength=len(data)))
    blob.extend(data)
    accessors.append(dict(bufferView=len(views)-1, componentType=component, count=len(values)//dims, type={1:'SCALAR',2:'VEC2',3:'VEC3',4:'VEC4',16:'MAT4'}[dims], **extra))
    return len(accessors)-1
pos = add([-0.8,-0.6,0.6, 0.8,-0.6,0.6, 0,0.9,0, 0,-0.6,-0.8], 'f',3,min=[-0.8,-0.6,-0.8],max=[0.8,0.9,0.6])
uv = add([0,0,1,0,0.5,1,1,1],'f',2)
color = add([0.1,0.5,1,1, 0.2,0.9,0.6,1, 1,0.5,0.1,1, 0.7,0.3,1,1], 'f',4)
indices = add([0,1,2, 1,3,2, 3,0,2, 0,3,1], 'H',1,5123)
morph = add([0,0,0, 0,0,0, 0,0.8,0, 0,0,0], 'f',3)
times = add([0,1,2], 'f',1,min=[0],max=[2])
rotation = add([0,0,0,1, 0,1,0,0, 0,0,0,-1], 'f',4)
weights = add([0,1,0], 'f',1)
joints = add([0,0,0,0]*4,'H',4,5123)
skin_weights = add([1,0,0,0]*4,'f',4)
ibm = add([1,0,0,0,0,1,0,0,0,0,1,0,0,0,0,1],'f',16)
def chunk(kind, data):
    return struct.pack('>I',len(data))+kind+data+struct.pack('>I',zlib.crc32(kind+data)&0xffffffff)
png = b'\x89PNG\r\n\x1a\n'+chunk(b'IHDR',struct.pack('>IIBBBBB',2,2,8,6,0,0,0))+chunk(b'IDAT',zlib.compress(bytes([0]+[255,255,255,255]*2+[0]+[160,180,255,255]*2)))+chunk(b'IEND',b'')
base = dict(asset=dict(version='2.0',generator='TekesUI deterministic MIT fixture'),scene=0,
    scenes=[dict(nodes=[0])],nodes=[dict(mesh=0)],
    meshes=[dict(weights=[0],primitives=[dict(attributes=dict(POSITION=pos,COLOR_0=color,TEXCOORD_0=uv),indices=indices,material=0,targets=[dict(POSITION=morph)])])],
    animations=[dict(name='Turn',samplers=[dict(input=times,output=rotation)],channels=[dict(sampler=0,target=dict(node=0,path='rotation'))]),
        dict(name='Stretch',samplers=[dict(input=times,output=weights)],channels=[dict(sampler=0,target=dict(node=0,path='weights'))])],
    materials=[dict(pbrMetallicRoughness=dict(baseColorTexture=dict(index=0)))],
    textures=[dict(source=0)],images=[dict(uri='data:image/png;base64,'+base64.b64encode(png).decode())],
    buffers=[dict(byteLength=len(blob))],bufferViews=views,accessors=accessors)
def glb(document, name):
    j = json.dumps(document,separators=(',',':')).encode(); j+=b' '*((-len(j))%4)
    b = bytes(blob); b+=b'\0'*((-len(b))%4)
    result=struct.pack('<III',0x46546c67,2,12+8+len(j)+8+len(b))+struct.pack('<II',len(j),0x4e4f534a)+j+struct.pack('<II',len(b),0x004e4942)+b
    (root/name).write_bytes(result)
glb(base,'animated-tetra.glb')
base['buffers'][0]['uri']='animated-tetra.bin'
(root/'animated-tetra.gltf').write_text(json.dumps(base,indent=2)+'\n')
(root/'animated-tetra.bin').write_bytes(blob)
del base['buffers'][0]['uri']
base['nodes']=[dict(mesh=0,skin=0),dict()]
base['scenes'][0]['nodes']=[0,1]
base['skins']=[dict(joints=[1],inverseBindMatrices=ibm)]
base['meshes'][0]['primitives'][0]['attributes'].update(JOINTS_0=joints,WEIGHTS_0=skin_weights)
base['animations'][0]['channels'][0]['target']['node']=1
glb(base,'skinned-tetra.glb')
