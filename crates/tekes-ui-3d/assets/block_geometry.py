"""Small offline glTF authoring helper. Original geometry, no runtime dependency."""
import json,math,struct,io
from collections import defaultdict
from PIL import Image,ImageDraw
class Asset:
    def __init__(self):
        self.blob=bytearray();self.views=[];self.accessors=[];self.materials=[];self.meshes=[];self.nodes=[];self.images=[];self.textures=[]
    def material(self,name,color,texture=None,masked=False):
        def linear(v):return v/255/12.92 if v<=10.31475 else ((v/255+.055)/1.055)**2.4
        pbr=dict(baseColorFactor=[linear(v) for v in color]+[1],metallicFactor=0,roughnessFactor=.8)
        if texture is not None:pbr['baseColorTexture']=dict(index=texture)
        m=dict(name=name,pbrMetallicRoughness=pbr,doubleSided=True)
        if masked:m.update(alphaMode='MASK',alphaCutoff=.4)
        self.materials.append(m);return len(self.materials)-1
    def image(self,image):
        out=io.BytesIO();image.save(out,format='PNG');data=out.getvalue()
        while len(self.blob)%4:self.blob.append(0)
        self.views.append(dict(buffer=0,byteOffset=len(self.blob),byteLength=len(data)));self.blob.extend(data)
        self.images.append(dict(bufferView=len(self.views)-1,mimeType='image/png'));self.textures.append(dict(source=len(self.images)-1));return len(self.textures)-1
    def accessor(self,values,dims,integer=False,bounds=False):
        while len(self.blob)%4:self.blob.append(0)
        data=struct.pack('<'+('I' if integer else 'f')*len(values),*values)
        self.views.append(dict(buffer=0,byteOffset=len(self.blob),byteLength=len(data)));self.blob.extend(data)
        a=dict(bufferView=len(self.views)-1,componentType=5125 if integer else 5126,count=len(values)//dims,type={1:'SCALAR',2:'VEC2',3:'VEC3',4:'VEC4'}[dims])
        if bounds:a.update(min=[min(values[c::dims]) for c in range(dims)],max=[max(values[c::dims]) for c in range(dims)])
        self.accessors.append(a);return len(self.accessors)-1
    def finish(self):
        d=dict(asset=dict(version='2.0',generator='TekesUI original waterfront district'),scene=0,scenes=[dict(nodes=list(range(len(self.nodes))))],nodes=self.nodes,meshes=self.meshes,materials=self.materials,images=self.images,textures=self.textures,buffers=[dict(byteLength=len(self.blob))],bufferViews=self.views,accessors=self.accessors)
        j=json.dumps(d,separators=(',',':')).encode();j+=b' '*((-len(j))%4);b=bytes(self.blob);b+=b'\0'*((-len(b))%4)
        return struct.pack('<III',0x46546c67,2,28+len(j)+len(b))+struct.pack('<II',len(j),0x4e4f534a)+j+struct.pack('<II',len(b),0x004e4942)+b
class Geometry:
    def __init__(self,asset):self.asset=asset;self.groups=defaultdict(lambda:[[],[],[],[],[]])
    def quad(self,pts,mat,color=(255,255,255),uv=None,normals=None):
        p,n,t,c,indices=self.groups[mat];start=len(p)//3
        u=[pts[1][i]-pts[0][i] for i in range(3)];v=[pts[2][i]-pts[0][i] for i in range(3)]
        normal=[u[1]*v[2]-u[2]*v[1],u[2]*v[0]-u[0]*v[2],u[0]*v[1]-u[1]*v[0]];l=math.sqrt(sum(x*x for x in normal));normal=[x/l for x in normal]
        linear=lambda v:v/255/12.92 if v<=10.31475 else ((v/255+.055)/1.055)**2.4
        rgba=[linear(v) for v in color]+[1]
        for i,point in enumerate(pts):p.extend(point);n.extend(normals[i] if normals else normal);t.extend(uv[i] if uv else [(0,1),(1,1),(1,0),(0,0)][i]);c.extend(rgba)
        indices.extend([start,start+1,start+2,start,start+2,start+3])
    def box(self,x,y,z,w,h,d,mat,color=(255,255,255),facade=None):
        a,b=x-w/2,x+w/2;c,e=y-h/2,y+h/2;f,g=z-d/2,z+d/2
        faces=[[(a,c,g),(b,c,g),(b,e,g),(a,e,g)],[(b,c,f),(a,c,f),(a,e,f),(b,e,f)],[(b,c,g),(b,c,f),(b,e,f),(b,e,g)],[(a,c,f),(a,c,g),(a,e,g),(a,e,f)]]
        for index,pts in enumerate(faces):
            width=w if index<2 else d
            uv=[(0,h/10),(width/5,h/10),(width/5,0),(0,0)] if facade is not None else None
            self.quad(pts,facade if facade is not None else mat,color,uv)
        self.quad([(a,e,g),(b,e,g),(b,e,f),(a,e,f)],mat,color)
        self.quad([(a,c,f),(b,c,f),(b,c,g),(a,c,g)],mat,color)
    def ellipsoid(self,x,y,z,rx,ry,rz,mat,color=(255,255,255),sides=16,rings=8):
        for row in range(rings):
            a=.015+row*(math.pi-.030)/rings;b=.015+(row+1)*(math.pi-.030)/rings
            for i in range(sides):
                p=i*math.tau/sides;q=(i+1)*math.tau/sides
                angles=[(a,p),(a,q),(b,q),(b,p)]
                pts=[(x+rx*math.sin(t)*math.cos(f),y+ry*math.cos(t),z+rz*math.sin(t)*math.sin(f)) for t,f in angles]
                ns=[]
                for point in pts:
                    n=[(point[0]-x)/(rx*rx),(point[1]-y)/(ry*ry),(point[2]-z)/(rz*rz)];length=math.sqrt(sum(v*v for v in n));ns.append([v/length for v in n])
                self.quad(pts,mat,color,normals=ns)
    def cylinder(self,x,y,z,r,h,mat,color=(255,255,255),axis='y'):
        def point(t,v):
            if axis=='x':return (x+v,y+r*math.cos(t),z+r*math.sin(t))
            return (x+r*math.cos(t),y+v,z+r*math.sin(t))
        for i in range(16):
            a,b=i*math.tau/16,(i+1)*math.tau/16
            self.quad([point(a,-h/2),point(b,-h/2),point(b,h/2),point(a,h/2)],mat,color)
        for sign in [-1,1]:
            center=(x+sign*h/2,y,z) if axis=='x' else (x,y+sign*h/2,z)
            for i in range(0,16,2):
                self.quad([center,point(i*math.tau/16,sign*h/2),point((i+1)*math.tau/16,sign*h/2),point((i+2)*math.tau/16,sign*h/2)],mat,color)
    def beam(self,start,end,width,mat,color):
        direction=[end[i]-start[i] for i in range(3)];length=math.sqrt(sum(v*v for v in direction));v=[x/length for x in direction]
        ref=[0,1,0] if abs(v[1])<.9 else [1,0,0]
        u=[v[1]*ref[2]-v[2]*ref[1],v[2]*ref[0]-v[0]*ref[2],v[0]*ref[1]-v[1]*ref[0]];l=math.sqrt(sum(x*x for x in u));u=[x/l*width/2 for x in u]
        w=[v[1]*u[2]-v[2]*u[1],v[2]*u[0]-v[0]*u[2],v[0]*u[1]-v[1]*u[0]]
        rings=[[[p[i]+a*u[i]+b*w[i] for i in range(3)] for a,b in [(1,1),(-1,1),(-1,-1),(1,-1)]] for p in [start,end]]
        for j in range(4):self.quad([rings[0][j],rings[0][(j+1)%4],rings[1][(j+1)%4],rings[1][j]],mat,color)
    def finish(self,name):
        primitives=[]
        for mat,(p,n,uv,c,idx) in self.groups.items():
            a=self.asset
            primitives.append(dict(attributes=dict(POSITION=a.accessor(p,3,bounds=True),NORMAL=a.accessor(n,3),TEXCOORD_0=a.accessor(uv,2),COLOR_0=a.accessor(c,4)),indices=a.accessor(idx,1,True),material=mat))
        self.asset.meshes.append(dict(name=name,primitives=primitives));self.asset.nodes.append(dict(name=name,mesh=len(self.asset.meshes)-1))
def facade(style):
    im=Image.new('RGB',(512,1024));d=ImageDraw.Draw(im)
    colors=[((186,195,191),(99,142,162)),((165,116,88),(93,115,122)),((222,213,190),(110,135,144))]
    frame,glass=colors[style]
    for y in range(0,1024,32):
        for x in range(0,512,24):
            d.rectangle((x,y,x+23,y+31),fill=frame)
            for row in range(4,28):
                tint=(x//24*7+y//32*13)%17-8
                d.line((x+3,y+row,x+20,y+row),fill=tuple(max(0,min(255,v+tint+int((row-16)*.8))) for v in glass))
            d.line((x+2,y+29,x+21,y+29),fill=tuple(max(0,v-22) for v in frame),width=2)
            d.line((x+4,y+5,x+19,y+5),fill=tuple(min(255,v+30) for v in glass),width=1)
            d.line((x+12,y+5,x+12,y+27),fill=frame,width=1)
    return im
