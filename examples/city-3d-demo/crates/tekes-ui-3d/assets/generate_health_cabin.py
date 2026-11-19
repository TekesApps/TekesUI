"""AI health cabin, modeled from user-supplied PDF pages 6 and 12-14.
Meters; overall shell 3.905 x 2.635 x 2.886, floor clearance .185.
Original demo geometry, furniture details approximate. Requires Pillow for signs.
No PDF images, contact details or third-party surroundings are embedded.
"""
from pathlib import Path
from collections import defaultdict
import io
import json
import math
import struct
from PIL import Image, ImageDraw, ImageFont


def build():
    blob=bytearray();views=[];accessors=[];meshes=[];nodes=[];materials=[];images=[];textures=[]
    def acc(values,dims,fmt='f',component=5126,bounds=False):
        while len(blob)%4:blob.append(0)
        data=struct.pack('<'+fmt*len(values),*values);views.append(dict(buffer=0,byteOffset=len(blob),byteLength=len(data)));blob.extend(data)
        a=dict(bufferView=len(views)-1,componentType=component,count=len(values)//dims,type={1:'SCALAR',2:'VEC2',3:'VEC3',4:'VEC4'}[dims])
        if bounds:a.update(min=[min(values[c::dims]) for c in range(dims)],max=[max(values[c::dims]) for c in range(dims)])
        accessors.append(a);return len(accessors)-1
    def material(name,rgb,alpha=1,texture=None):
        def linear(v):return v/255/12.92 if v<=10.31475 else ((v/255+.055)/1.055)**2.4
        pbr=dict(baseColorFactor=[linear(v) for v in rgb]+[alpha],metallicFactor=0,roughnessFactor=.5)
        if texture is not None:pbr['baseColorTexture']=dict(index=texture)
        m=dict(name=name,pbrMetallicRoughness=pbr,doubleSided=True)
        if alpha<1:m['alphaMode']='BLEND'
        if texture is not None:m.update(alphaMode='MASK',alphaCutoff=.3)
        materials.append(m);return len(materials)-1
    def label_texture(text,width=1024,height=180):
        im=Image.new('RGBA',(width,height),(255,255,255,0));d=ImageDraw.Draw(im)
        font=ImageFont.truetype('/System/Library/Fonts/STHeiti Medium.ttc',int(height*.72))
        rect=d.textbbox((0,0),text,font=font);d.text(((width-rect[2])/2,(height-(rect[3]-rect[1]))/2-rect[1]),text,font=font,fill=(23,28,31,255))
        stream=io.BytesIO();im.save(stream,format='PNG');data=stream.getvalue()
        while len(blob)%4:blob.append(0)
        views.append(dict(buffer=0,byteOffset=len(blob),byteLength=len(data)));blob.extend(data)
        images.append(dict(bufferView=len(views)-1,mimeType='image/png'));textures.append(dict(source=len(images)-1))
        return len(textures)-1
    white=material('Warm white painted shell',(243,241,233));inside=material('Interior wall',(228,226,214))
    black=material('Black anodized frames',(30,35,37));metal=material('Brushed aluminum',(166,178,181))
    floor=material('Light gray PVC floor',(179,184,180));dark=material('Equipment display',(24,52,65))
    teal=material('Medical equipment teal',(68,144,147));red=material('AED emergency cabinet',(187,48,46))
    glass=material('Clear blue glass - alpha blend',(179,219,226),.17)
    highlight=material('Subtle glass reflection',(242,249,247),.18)
    sign=material('AI健康小屋 signage',(255,255,255),texture=label_texture('AI健康小屋'))
    chairmat=material('Chair upholstery',(133,154,150));led=material('LED diffusers',(255,255,247))
    platform=material('Display plinth',(197,203,203))
    class Geometry:
        def __init__(self):self.groups=defaultdict(lambda:[[],[],[],[]])
        def quad(self,points,mat,ns=None,uv=None):
            ps,normal,tex,idx=self.groups[mat];start=len(ps)//3
            u=[points[1][i]-points[0][i] for i in range(3)];v=[points[2][i]-points[0][i] for i in range(3)]
            n=[u[1]*v[2]-u[2]*v[1],u[2]*v[0]-u[0]*v[2],u[0]*v[1]-u[1]*v[0]];length=math.sqrt(sum(x*x for x in n));n=[x/length for x in n]
            for i,p in enumerate(points):ps.extend(p);normal.extend(ns[i] if ns else n);tex.extend(uv[i] if uv else [(0,1),(1,1),(1,0),(0,0)][i])
            idx.extend([start,start+1,start+2,start,start+2,start+3])
        def box(self,x,y,z,w,h,d,mat):
            a,b=x-w/2,x+w/2;c,e=y-h/2,y+h/2;f,g=z-d/2,z+d/2
            for pts in [[(a,c,g),(b,c,g),(b,e,g),(a,e,g)],[(b,c,f),(a,c,f),(a,e,f),(b,e,f)],[(b,c,g),(b,c,f),(b,e,f),(b,e,g)],[(a,c,f),(a,c,g),(a,e,g),(a,e,f)],[(a,e,g),(b,e,g),(b,e,f),(a,e,f)],[(a,c,f),(b,c,f),(b,c,g),(a,c,g)]]:self.quad(pts,mat)
        def cylinder(self,x,y,z,r,length,mat,axis='y',segments=32):
            def point(t,h):
                p=[r*math.cos(t),h,r*math.sin(t)]
                if axis=='x':p=[h,p[0],p[2]]
                if axis=='z':p=[p[0],p[2],h]
                return (x+p[0],y+p[1],z+p[2])
            for i in range(segments):
                a,b=i*math.tau/segments,(i+1)*math.tau/segments
                self.quad([point(a,-length/2),point(b,-length/2),point(b,length/2),point(a,length/2)],mat)
            # Caps fan triangulated through quads with two different center points.
            for h in [-length/2,length/2]:
                if axis=='y':center=(x,y+h,z)
                elif axis=='x':center=(x+h,y,z)
                else:center=(x,y,z+h)
                for i in range(0,segments,2):
                    self.quad([center,point(i*math.tau/segments,h),point((i+1)*math.tau/segments,h),point((i+2)*math.tau/segments,h)],mat)
        def node(self,name,translation=None,rotation=None):
            primitives=[]
            for mat,(ps,ns,uv,idx) in self.groups.items():
                primitives.append(dict(attributes=dict(POSITION=acc(ps,3,bounds=True),NORMAL=acc(ns,3),TEXCOORD_0=acc(uv,2)),indices=acc(idx,1,'I',5125),material=mat))
            meshes.append(dict(name=name,primitives=primitives));n=dict(name=name,mesh=len(meshes)-1)
            if translation:n['translation']=translation
            if rotation:n['rotation']=rotation
            nodes.append(n);return len(nodes)-1
    def rounded(w,h,r,y0):
        pts=[]
        for cx,cy,a in [(w/2-r,y0+r,-90),(w/2-r,y0+h-r,0),(-w/2+r,y0+h-r,90),(-w/2+r,y0+r,180)]:
            for i in range(17):
                t=math.radians(a+i*90/16);pts.append((cx+r*math.cos(t),cy+r*math.sin(t)))
        return pts
    W,D,H,Y=3.905,2.635,2.886,.185;F=D/2
    outer=rounded(W,H,.27,Y);inner=rounded(W-.18,H-.18,.18,Y+.09)
    body=Geometry()
    for i in range(len(outer)):
        j=(i+1)%len(outer);a,b=outer[i],outer[j];c,d=inner[i],inner[j]
        body.quad([(a[0],a[1],-F),(b[0],b[1],-F),(b[0],b[1],F),(a[0],a[1],F)],white)
        body.quad([(c[0],c[1],F-.07),(d[0],d[1],F-.07),(d[0],d[1],-F+.08),(c[0],c[1],-F+.08)],inside)
        body.quad([(a[0],a[1],F),(b[0],b[1],F),(d[0],d[1],F),(c[0],c[1],F)],white)
        # Rounded back face fan, entirely behind the interior wall.
        body.quad([(0,Y+H/2,-F),(b[0],b[1],-F),(a[0],a[1],-F),((a[0]+b[0])*.499,Y+H/2,-F)],white)
    body.box(0,Y+.09,0,W-.18,.08,D-.15,floor)
    body.box(0,Y+H/2,-F+.09,W-.22,H-.22,.055,inside)
    # White front lower panels and header; windows remain real openings.
    for x in [-1.14,1.14]:body.box(x,Y+.31,F-.025,1.22,.44,.075,white)
    body.box(0,2.69,F-.04,3.58,.54,.07,white)
    # Front framing, door clear opening 960 x 2200 mm.
    for x in [-1.79,-.53,.53,1.79]:body.box(x,1.55,F+.006,.045,1.72,.045,black)
    for side in [-1,1]:
        x=side*1.16
        for y in [.73,1.565,2.40]:body.box(x,y,F+.012,1.30,.043,.05,black)
        # Fixed lower glazing, a single thin pane rather than a solid block.
        body.quad([(x-.607,.755,F),(x+.607,.755,F),(x+.607,1.54,F),(x-.607,1.54,F)],glass)
    for x in [-.505,.505]:body.box(x,1.40,F+.036,.05,2.32,.065,black)
    body.box(0,2.51,F+.038,1.06,.20,.06,black)
    body.box(0,2.52,F+.074,.28,.045,.009,led)
    body.box(0,.23,F+.025,1.06,.065,.07,black)
    # Signature black contour above the glazing, with a central dropped sign band.
    outline=[(-1.79,2.81),(-1.69,2.95),(-.82,2.95),(-.53,2.73),(.53,2.73),(.82,2.95),(1.69,2.95),(1.79,2.81)]
    for (x0,y0),(x1,y1) in zip(outline,outline[1:]):
        dx,dy=x1-x0,y1-y0;l=math.hypot(dx,dy);nx,ny=-dy/l*.019,dx/l*.019
        body.quad([(x0-nx,y0-ny,F+.009),(x1-nx,y1-ny,F+.009),(x1+nx,y1+ny,F+.009),(x0+nx,y0+ny,F+.009)],black)
    body.quad([(-.65,2.77,F+.015),(.65,2.77,F+.015),(.65,2.97,F+.015),(-.65,2.97,F+.015)],sign)
    # Rear external AC unit, grille and exhaust opening.
    body.box(-1.12,.56,-F-.22,.78,.48,.38,white)
    for x in [-1.42,-.82]:body.box(x,.29,-F-.19,.035,.20,.34,metal)
    body.cylinder(-1.12,.56,-F-.421,.18,.012,black,'z')
    for i in range(-8,9):body.box(-1.12+i*.020,.56,-F-.432,.007,.37,.008,metal)
    body.cylinder(.05,2.73,-F-.008,.065,.018,metal,'z')
    # Casters and jack stands, 185 mm clearance.
    for x in [-1.56,1.56]:
        for z in [-.94,.94]:
            body.box(x,.155,z,.07,.06,.09,metal);body.cylinder(x,.075,z,.064,.046,black,'x')
            body.cylinder(x+.13,.08,z,.018,.13,metal)
            body.cylinder(x+.13,.018,z,.055,.014,metal)
    body.box(0,.065,F+.27,1.04,.13,.46,black)
    for i in range(-11,12):body.box(i*.042,.133,F+.26,.014,.003,.40,metal)
    body.node('Shell 3905 x 2635 x 2886 mm; front windows; rear AC')
    # Furniture positions follow PDF page 6; exact equipment is schematic.
    furniture=Geometry()
    furniture.box(-1.62,.77,-.10,.40,.065,1.74,white)
    furniture.box(-1.64,.47,-.10,.33,.54,1.68,inside)
    furniture.box(.10,.88,-.42,.96,.065,.55,white)
    for x in [-.30,.50]:
        for z in [-.61,-.23]:furniture.cylinder(x,.56,z,.018,.61,metal)
    for x,z in [(-.85,-.44),(.12,.33)]:
        furniture.box(x,.64,z,.43,.07,.43,chairmat)
        furniture.box(x,.92,z+.21,.43,.50,.055,chairmat)
        furniture.cylinder(x,.40,z,.027,.40,metal)
        for off in [-.16,.16]:furniture.box(x+off,.23,z,.022,.05,.40,black)
    furniture.box(1.56,1.02,-.65,.46,1.48,.94,white)
    for y in [.61,1.08,1.55]:furniture.box(1.315,y,-.65,.008,.018,.86,metal)
    furniture.box(1.57,1.25,.72,.34,.42,.13,red)
    furniture.box(1.57,1.25,.798,.24,.31,.012,white)
    furniture.box(1.57,1.25,.81,.13,.035,.008,red)
    furniture.box(1.57,1.25,.812,.035,.13,.008,red)
    # Generic diagnostic kiosk behind the table, not an invented real device.
    furniture.box(.10,.99,-.91,.30,1.42,.24,white)
    furniture.box(.10,1.53,-.77,.26,.34,.02,dark)
    furniture.box(.10,1.53,-.756,.18,.23,.005,teal)
    for x in [-1.18,1.18]:furniture.box(x,2.94,0,.025,.018,1.95,led)
    furniture.box(0,2.945,-.35,.60,.018,.42,led)
    furniture.node('PDF layout - desk, two chairs, cabinet, AED, diagnostic kiosk')
    # Door hinged on right jamb; animation opens inward.
    door=Geometry()
    for x in [-.935,-.025]:door.box(x,1.10,0,.032,2.20,.035,black)
    for y in [.025,2.175]:door.box(-.48,y,0,.94,.035,.035,black)
    door.quad([(-.916,.05,0),(-.044,.05,0),(-.044,2.15,0),(-.916,2.15,0)],glass)
    door.cylinder(-.82,1.03,.046,.009,.65,metal)
    door.box(-.82,.74,.028,.022,.025,.065,metal);door.box(-.82,1.32,.028,.022,.025,.065,metal)
    door_node=door.node('Inward-opening glass door, clear width 960 mm',[.48,.27,F+.053])
    window_nodes=[]
    for x in [-1.16,1.16]:
        window=Geometry()
        for y in [0,-.787]:window.box(0,y,0,1.273,.034,.034,black)
        for dx in [-.62,.62]:window.box(dx,-.3935,0,.034,.787,.034,black)
        window.quad([(-.60,-.766,0),(.60,-.766,0),(.60,-.02,0),(-.60,-.02,0)],glass)
        # A quiet diagonal highlight conveys the glazing plane without faking a photograph.
        window.quad([(-.47,-.75,.002),(-.43,-.75,.002),(.02,-.03,.002),(-.02,-.03,.002)],highlight)
        window_nodes.append(window.node('Top-hinged awning window',[x,2.375,F+.035],[-math.sin(.21),0,0,math.cos(.21)]))
    # Neutral stage; not the unrelated surroundings from the source render.
    stage=Geometry();stage.box(0,-.025,0,4.70,.05,3.85,platform);stage.node('Display ground')
    times=acc([0,1,2,3,4,5],1,bounds=True)
    channels=[];samplers=[]
    def animate(node,axis,angles):
        q=[]
        for angle in angles:
            v=[0.,0.,0.,math.cos(angle/2)];v[axis]=math.sin(angle/2);q.extend(v)
        samplers.append(dict(input=times,output=acc(q,4),interpolation='LINEAR'))
        channels.append(dict(sampler=len(samplers)-1,target=dict(node=node,path='rotation')))
    animate(door_node,1,[0,0,-1.22,-1.22,0,0])
    for node in window_nodes:animate(node,0,[-.42,-.68,-.68,-.42,-.42,-.42])
    doc=dict(asset=dict(version='2.0',generator='TekesUI3D original health cabin from supplied dimensioned PDF'),scene=0,scenes=[dict(nodes=list(range(len(nodes))))],nodes=nodes,meshes=meshes,materials=materials,images=images,textures=textures,
             animations=[dict(name='Open door and awning windows',samplers=samplers,channels=channels)],buffers=[dict(byteLength=len(blob))],bufferViews=views,accessors=accessors)
    j=json.dumps(doc,ensure_ascii=False,separators=(',',':')).encode();j+=b' '*((-len(j))%4);b=bytes(blob);b+=b'\0'*((-len(b))%4)
    glb=struct.pack('<III',0x46546c67,2,28+len(j)+len(b))+struct.pack('<II',len(j),0x4e4f534a)+j+struct.pack('<II',len(b),0x004e4942)+b
    return glb,dict(nodes=len(nodes),primitives=sum(len(m['primitives']) for m in meshes),shell_m=[W,D,H],clearance_m=Y)

if __name__=='__main__':
    data,stats=build();out=Path(__file__).with_name('health-cabin.glb');out.write_bytes(data);print(out,len(data),'bytes',stats)
