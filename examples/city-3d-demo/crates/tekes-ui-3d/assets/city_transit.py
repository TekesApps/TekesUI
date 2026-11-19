"""Original ground-level through line, station, catenary and detailed coaches.
One unit = four metres. Standalone authoring module, no licensed assets/logos.
"""
import math
from block_geometry import Geometry
from city_shopfronts import sign_image
from city_train import add_train_assets
DECK=0.0
RAIL=.10
WIRE=1.35

def strip(g,pa,pb,width,y,height,mat,color):
    x,_,z=pa;xx,_,zz=pb;d=math.hypot(xx-x,zz-z);nx,nz=(zz-z)/d,-(xx-x)/d
    q=[(x-nx*width/2,y,z-nz*width/2),(xx-nx*width/2,y,zz-nz*width/2),(xx+nx*width/2,y,zz+nz*width/2),(x+nx*width/2,y,z+nz*width/2)]
    g.quad(q,mat,color,normals=[(0,1,0)]*4)
    for a,b in [(q[0],q[1]),(q[2],q[3])]:g.quad([(a[0],y-height,a[2]),(b[0],y-height,b[2]),b,a],mat,color)

def add_transit(a,plain,headlights,taillights,routes):
    dark=(42,52,58);green=(98,164,55);silver=(198,207,211)
    # Visible through tracks are straight; hidden return legs live outside the
    # GPU route window, so no tiny-radius U-turn or loop through the city.
    tracks=[[[x,RAIL,-12],[x,RAIL,12]] for x in [9.4,11.2]]
    g=Geometry(a)
    g.box(10.3,-.24,0,3.3,.48,24.5,plain,(158,169,152))
    for points in tracks:
        pole_distance=0
        for pa,pb in zip(points,points[1:]):
            x,_,z=pa;xx,_,zz=pb;d=math.hypot(xx-x,zz-z);nx,nz=(zz-z)/d,-(xx-x)/d
            # Ground reservation between the road and waterfront.
            strip(g,pa,pb,.93,DECK,.14,plain,(157,164,158))
            strip(g,pa,pb,.70,DECK+.025,.025,plain,(108,115,108))
            for i in range(max(1,math.ceil(d/.18))):
                t=(i+.5)/max(1,math.ceil(d/.18));sx,sz=x+(xx-x)*t,z+(zz-z)*t
                g.beam((sx-nx*.31,DECK+.045,sz-nz*.31),(sx+nx*.31,DECK+.045,sz+nz*.31),.035,plain,(99,92,77))
            for side in [-1,1]:
                ra=(x+side*nx*.1794,0,z+side*nz*.1794);rb=(xx+side*nx*.1794,0,zz+side*nz*.1794)
                # Actual paired steel rails: base, narrow web, polished rail head.
                strip(g,ra,rb,.043,RAIL-.050,.008,plain,(92,104,111))
                strip(g,ra,rb,.010,RAIL-.012,.040,plain,(114,127,135))
                strip(g,ra,rb,.027,RAIL,.014,plain,silver)
            # Continuous contact and messenger wires, plus short vertical droppers.
            g.beam((x,WIRE,z),(xx,WIRE,zz),.007,plain,(62,68,73))
            g.beam((x,WIRE+.16,z),(xx,WIRE+.16,zz),.006,plain,(79,85,89))
            for step in range(math.ceil(d/2.4)):
                t=(step+.5)/math.ceil(d/2.4);wx,wz=x+(xx-x)*t,z+(zz-z)*t
                pole_side=-1 if x < 10.3 else 1
                px,pz=wx+pole_side*.52,wz
                g.box(px,(DECK+WIRE+.25)/2,pz,.035,WIRE+.25-DECK,.035,plain,(119,132,139))
                g.beam((px,WIRE+.19,pz),(wx-pole_side*.12,WIRE+.19,wz),.025,plain,(134,145,149))
                g.beam((wx,WIRE,wz),(wx,WIRE+.19,wz),.009,plain,(82,91,99))
    g.finish('transit-ground-double-track')
    g=Geometry(a);sx=10.3;top=.34
    g.box(sx,top-.10,0,1.0,.20,16.1,plain,(196,195,181))
    for x in [sx-.46,sx+.46]:g.box(x,top+.008,0,.05,.014,15.8,plain,(232,199,65))
    for end in [-1,1]:
        for j in range(4):g.box(sx,.14+(4-j)*.025,end*(8.12+j*.20),.65,(4-j)*.05,.20,plain,(188,189,178))
    for end in [-1,1]:g.box(8.7,.13,end*8.85,4.1,.10,.55,plain,(184,183,167))
    for z in [-7.,-4.8,-2.4,0,2.4,4.8,7.]:g.box(sx,(top+WIRE+.08)/2,z,.035,WIRE+.08-top,.035,plain,(93,110,107))
    g.box(sx,WIRE+.12,0,.97,.065,15.2,plain,(81,111,96))
    for z in [-5.,-2.5,2.5,5.]:
        g.box(sx+.24,top+.16,z,.28,.05,.65,plain,(159,132,88))
        for dz in [-.22,.22]:g.box(sx+.24,top+.075,z+dz,.04,.15,.04,plain,(93,110,107))
    for z in [-6,-3,0,3,6]:g.box(sx,WIRE+.075,z,.62,.018,.035,headlights)
    sign=a.material('Original riverside station',(255,255,255),a.image(sign_image('RIVERSIDE',green)))
    g.box(sx+.15,WIRE-.13,3.,.045,.22,1.1,sign)
    g.finish('transit-island-station')
    add_train_assets(a,plain,headlights,taillights)
    first=len(routes)
    for i,x in enumerate([9.4,11.2]):
        z0,z1=(28,-28) if i==0 else (-28,28)
        routes.append(dict(name='Through railway '+('northbound' if i==0 else 'southbound'),points=[[x,RAIL,z0],[x,RAIL,z1],[x,RAIL-2,z1-.01]]))
    return [dict(route=first+i,distance=28,speed=2.1,color=[1,1,1],scale=1,phase=0 if i==0 else 12) for i in range(2)]
