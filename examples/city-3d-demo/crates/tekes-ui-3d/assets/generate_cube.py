"""MIT procedural 3x3 cube, glTF 2.0. No downloaded geometry or textures."""
from pathlib import Path
import math, struct, json
blob=bytearray(); views=[]; accessors=[]; meshes=[]
def accessor(values, dims, kind='f', component=5126, bounds=False):
    while len(blob)%4: blob.append(0)
    data=struct.pack('<'+kind*len(values),*values)
    views.append(dict(buffer=0,byteOffset=len(blob),byteLength=len(data))); blob.extend(data)
    a=dict(bufferView=len(views)-1,componentType=component,count=len(values)//dims,type={1:'SCALAR',3:'VEC3',4:'VEC4'}[dims])
    if bounds:
        a.update(min=[min(values[c::dims]) for c in range(dims)],max=[max(values[c::dims]) for c in range(dims)])
    accessors.append(a); return len(accessors)-1

def linear(rgb):
    def f(v):
        v=v/255.; return v/12.92 if v<=0.04045 else ((v+0.055)/1.055)**2.4
    return [f(v) for v in rgb]+[1.]
colors={(0,1):linear((205,38,44)),(0,-1):linear((255,135,30)),(1,1):linear((242,242,235)),(1,-1):linear((245,208,25)),(2,1):linear((0,155,91)),(2,-1):linear((35,90,215))}
black=linear((23,25,29)); h=0.47; bevel=0.045; inner=h-bevel
nodes=[dict(name='Magic cube',children=[1]),dict(name='Top layer',children=[])]
for x in [-1,0,1]:
 for y in [-1,0,1]:
  for z in [-1,0,1]:
   if x==y==z==0: continue
   pos=[]; normals=[]; rgba=[]; indices=[]
   def face(points,normal,color):
       length=math.sqrt(sum(v*v for v in normal)); n=[v/length for v in normal]
       start=len(pos)//3
       for p in points: pos.extend(p); normals.extend(n); rgba.extend(color)
       for i in range(1,len(points)-1):
           a,b,c=points[0],points[i],points[i+1]
           u=[b[k]-a[k] for k in range(3)];v=[c[k]-a[k] for k in range(3)]
           cross=[u[1]*v[2]-u[2]*v[1],u[2]*v[0]-u[0]*v[2],u[0]*v[1]-u[1]*v[0]]
           tri=[start,start+i,start+i+1]
           if sum(cross[k]*n[k] for k in range(3))<0: tri[1],tri[2]=tri[2],tri[1]
           indices.extend(tri)
   # Six flat panels, twelve bevel edges, eight bevel corners.
   for axis in range(3):
    others=[i for i in range(3) if i!=axis]
    for sign in [-1,1]:
       points=[]
       for a,b in [(-1,-1),(1,-1),(1,1),(-1,1)]:
           p=[0.,0.,0.];p[axis]=sign*h;p[others[0]]=a*inner;p[others[1]]=b*inner;points.append(p)
       n=[0,0,0];n[axis]=sign;face(points,n,black)
   for a,b in [(0,1),(0,2),(1,2)]:
    c=3-a-b
    for sa in [-1,1]:
     for sb in [-1,1]:
       points=[]
       for pa,pb,pc in [(h,inner,-inner),(inner,h,-inner),(inner,h,inner),(h,inner,inner)]:
           p=[0,0,0];p[a]=sa*pa;p[b]=sb*pb;p[c]=pc;points.append(p)
       n=[0,0,0];n[a]=sa;n[b]=sb;face(points,n,black)
   for sx in [-1,1]:
    for sy in [-1,1]:
     for sz in [-1,1]:
       face([[sx*h,sy*inner,sz*inner],[sx*inner,sy*h,sz*inner],[sx*inner,sy*inner,sz*h]],[sx,sy,sz],black)
   # Rounded colored stickers only on the six exterior cube faces.
   for axis,coordinate in enumerate([x,y,z]):
    if coordinate==0: continue
    others=[i for i in range(3) if i!=axis];points=[];half=0.375;radius=0.055
    for cx,cy,theta in [(half-radius,half-radius,0),(-half+radius,half-radius,90),(-half+radius,-half+radius,180),(half-radius,-half+radius,270)]:
     for step in range(5):
       t=math.radians(theta+step*22.5);p=[0.,0.,0.];p[axis]=coordinate*(h+0.007)
       p[others[0]]=cx+radius*math.cos(t);p[others[1]]=cy+radius*math.sin(t);points.append(p)
    n=[0,0,0];n[axis]=coordinate;face(points,n,colors[(axis,coordinate)])
   mesh=dict(name=f'Cubie {x},{y},{z}',primitives=[dict(attributes=dict(POSITION=accessor(pos,3,bounds=True),NORMAL=accessor(normals,3),COLOR_0=accessor(rgba,4)),indices=accessor(indices,1,'H',5123))])
   meshes.append(mesh); index=len(nodes);nodes.append(dict(name=mesh['name'],mesh=len(meshes)-1,translation=[x,y,z]))
   nodes[1 if y==1 else 0]['children'].append(index)
times=accessor([0,0.5,1.5,2,3,4],1,bounds=True)
rotations=accessor([v for angle in [0,0,math.pi/2,math.pi/2,0,0] for v in [0,math.sin(angle/2),0,math.cos(angle/2)]],4)
d=dict(asset=dict(version='2.0',generator='TekesUI3D MIT procedural cube'),scene=0,scenes=[dict(nodes=[0])],nodes=nodes,meshes=meshes,
 animations=[dict(name='Top layer quarter turn',samplers=[dict(input=times,output=rotations,interpolation='LINEAR')],channels=[dict(sampler=0,target=dict(node=1,path='rotation'))])],
 buffers=[dict(byteLength=len(blob))],bufferViews=views,accessors=accessors)
j=json.dumps(d,separators=(',',':')).encode();j+=b' '*((-len(j))%4);b=bytes(blob);b+=b'\0'*((-len(b))%4)
out=Path(__file__).with_name('rubiks-cube.glb');out.write_bytes(struct.pack('<III',0x46546c67,2,12+8+len(j)+8+len(b))+struct.pack('<II',len(j),0x4e4f534a)+j+struct.pack('<II',len(b),0x004e4942)+b)
print(f'{out}: {out.stat().st_size} bytes, {len(meshes)} cubies, 54 stickers')
