"""Original E235-inspired geometry. Reference: JR East official E235 vehicle page.
No photographs, logos, commercial models or copied textures are bundled.
Shell approx 19.3 x 2.95 metres; three-coach demonstration formation.
"""
import math,random
from PIL import Image,ImageDraw
from block_geometry import Geometry
from city_train_liveries import YAMANOTE_E235
Z_SCALE=1.4
DOORS=[-1.20,-.40,.40,1.20]

def stretch(g):
    for positions,normals,_,_,_ in g.groups.values():
        for i in range(2,len(positions),3):positions[i]*=Z_SCALE
        for i in range(0,len(normals),3):
            n=[normals[i],normals[i+1],normals[i+2]/Z_SCALE];l=math.sqrt(sum(v*v for v in n));normals[i:i+3]=[v/l for v in n]

def material(a,name,rgb,night=0):
    m=a.material(name,rgb)
    if night:
        a.materials[m]['emissiveFactor']=[night*(v/255)**2.2 for v in rgb]
    return m

def round_profile(w,y0,y1,r):
    out=[]
    for cx,cy,start in [(w/2-r,y1-r,0),(-w/2+r,y1-r,90),(-w/2+r,y0+r,180),(w/2-r,y0+r,270)]:
        for j in range(7):
            t=math.radians(start+j*15);out.append((cx+r*math.cos(t),cy+r*math.sin(t)))
    return out

def panel(g,x,z,width,y0,y1,mat,color=(255,255,255),depth=.018):
    g.box(x,(y0+y1)/2,z,depth,y1-y0,width,mat,color)

def add_train_assets(a,plain,headlights,taillights,livery=YAMANOTE_E235):
    silver=livery.stainless;green=livery.line_green;dark=(40,50,56)
    steel=material(a,'Original brushed stainless train shell',livery.stainless)
    glazing=material(a,'Train transparent glazing',(100,138,146))
    a.materials[glazing].update(alphaMode='BLEND');a.materials[glazing]['pbrMetallicRoughness']['baseColorFactor'][3]=.40
    # Shared night-only glow for every passenger pane, including both door sides.
    a.materials[glazing]['emissiveFactor']=[1.0,.94,.80]
    cab_glazing=material(a,'Train cab tinted windshield',(35,55,62))
    a.materials[cab_glazing].update(alphaMode='BLEND');a.materials[cab_glazing]['pbrMetallicRoughness']['baseColorFactor'][3]=.78
    rubber=material(a,'Train glazing and door rubber',(29,39,44))
    trim=material(a,'Train extruded window and door frames',livery.frame)
    green_m=material(a,'E235-style green vertical door areas',green)
    # Original lower-front dither gradient, generated rather than sampled.
    im=Image.new('RGB',(256,128),(28,40,42));pix=im.load()
    for y in range(128):
        for x in range(256):
            q=max(0,min(1,(y-28)/72));rad=q*3.5
            if (x%7-3)**2+(y%7-3)**2<rad*rad:pix[x,y]=green
    front=a.material('Original E235-inspired front dither',(255,255,255),a.image(im))
    led=material(a,'Train interior warm LED diffusers',(255,243,213),1.0)
    seat=material(a,'Train green longitudinal benches',(125,165,78),.38)
    floor=material(a,'Train interior floor',(180,188,185),.65)
    wall=material(a,'Train interior wall',(224,224,210),.85)
    skin=[material(a,'Passenger skin '+str(i),c,.50) for i,c in enumerate([(221,174,139),(173,119,85),(239,198,162),(115,78,58)])]
    coats=[material(a,'Passenger clothes '+str(i),c,.40) for i,c in enumerate([(47,72,91),(153,68,59),(195,173,128),(65,100,78),(119,89,130)])]
    hair=[material(a,'Passenger hair '+str(i),c,.08) for i,c in enumerate([(34,32,32),(80,56,43),(160,150,137)])]
    pants=material(a,'Passenger trousers',(49,56,63),.08)

    def door_leaf(g,x,z):
        # Build around a genuine pane opening: no opaque green/black backing.
        side=1 if x>0 else -1
        for y0,y1 in [(.31,.59),(.825,.89)]:panel(g,x,z,.118,y0,y1,green_m,depth=.024)
        for dz in [-.04625,.04625]:panel(g,x,z+dz,.0255,.59,.825,green_m,depth=.024)
        for yy in [.5975,.8175]:panel(g,x+side*.014,z,.073,yy-.0075,yy+.0075,rubber,depth=.012)
        for dz in [-.032,.032]:panel(g,x+side*.014,z+dz,.006,.59,.825,rubber,depth=.012)
        panel(g,x+side*.022,z,.061,.605,.810,glazing,depth=.010)

    def cab(g,sign):
        z=sign*1.69
        # Rounded rectangular green surround, black interior, no pointed nose.
        outer=round_profile(.734,.24,1.005,.075);inner=round_profile(.698,.258,.987,.061)
        for j in range(len(outer)):
            k=(j+1)%len(outer);o,p=outer[j],outer[k];u,v=inner[j],inner[k]
            g.quad([(o[0],o[1],z),(p[0],p[1],z),(v[0],v[1],z+sign*.012),(u[0],u[1],z+sign*.012)],green_m)
            g.quad([(o[0],o[1],sign*1.51),(p[0],p[1],sign*1.51),(p[0],p[1],z),(o[0],o[1],z)],steel)
        g.box(0,.40,z+sign*.015,.67,.285,.018,front)
        g.box(0,.945,z+sign*.015,.62,.078,.018,rubber)
        # A real windshield opening: transparent pane and recessed driver cabin.
        g.box(0,.727,z+sign*.017,.615,.35,.015,cab_glazing)
        for x in [-.327,.327]:g.box(x,.727,z+sign*.014,.036,.36,.022,rubber)
        for x in [-.245,.245]:g.box(x,.945,z+sign*.032,.044,.031,.018,headlights if sign>0 else taillights)
        g.box(0,.947,z+sign*.031,.32,.025,.018,plain,(117,171,112))
        for x in [-.16,.17]:g.beam((x-.06,.565,z+sign*.030),(x+.075,.66,z+sign*.030),.006,plain,dark)
        g.box(0,.205,sign*1.77,.18,.10,.14,plain,(65,77,80))
        g.box(0,.16,sign*1.65,.60,.10,.12,plain,(101,117,123))
        g.box(0,.42,sign*1.58,.50,.10,.19,plain,(60,72,78))
        g.box(-.15,.65,sign*1.46,.12,.25,.10,wall)

    def gangway(g,sign):
        z=sign*1.57
        g.box(0,.61,z,.70,.70,.06,steel)
        # Opaque full-depth bellows join at half the 4.70-unit coach pitch.
        # No glazing between cars: the gangway has an opaque end door and floor.
        reach=4.70/(2*Z_SCALE)
        depth=reach-1.57
        g.box(0,.57,sign*(1.57+depth/2),.31,.58,depth,rubber)
        g.box(0,.29,sign*(1.57+depth/2),.35,.035,depth,plain,(83,92,96))
        for k in range(7):
            zz=sign*(1.57+depth*k/6)
            for x in [-.164,.164]:g.box(x,.57,zz,.018,.59,.012,plain,(78,89,96))
            for yy in [.275,.865]:g.box(0,yy,zz,.34,.018,.012,plain,(78,89,96))
        g.box(0,.22,sign*1.77,.14,.08,.14,plain,(69,79,84))

    for kind in ['coach','cab-front','cab-rear']:
        g=Geometry(a)
        # Hollow shell: continuous solebar, roof and actual door/window apertures.
        g.box(0,.245,0,.70,.055,3.12,plain,(69,79,83))
        g.box(0,.275,0,.675,.028,3.10,floor)
        g.box(0,.99,0,.70,.045,3.10,steel)
        for x in [-.359,.359]:
            panel(g,x,0,3.10,.29,.31,steel)
            # Opaque body is assembled around apertures, never behind the glass.
            cuts=[-1.55,*[v for d in DOORS for v in [d-.12,d+.12]],1.55]
            for left,right in zip(cuts[::2],cuts[1::2]):
                panel(g,x,(left+right)/2,right-left,.31,.55,steel)
                panel(g,x,(left+right)/2,right-left,.82,.97,steel)
            for left,right in [(-1.08,-.52),(-.28,.28),(.52,1.08)]:
                z=(left+right)/2;w=right-left-.09
                # Taller sill/header and solid side piers keep the glass area
                # bounded; warm emission must not read as a continuous glass wall.
                for zz in [left+.0225,right-.0225]:panel(g,x,zz,.045,.55,.82,steel)
                for yy in [.55,.82]:panel(g,x*1.01,z,w,yy-.012,yy+.012,trim)
                for zz in [left+.051,right-.051,z]:panel(g,x*1.012,zz,.016,.55,.82,trim)
                panel(g,x*.99,z,w-.02,.565,.805,glazing,depth=.009)
            # End piers and narrow cab-side windows.
            for z in [-1.46,1.46]:panel(g,x,z,.15,.38,.97,steel)
            for d in DOORS:
                panel(g,x,d,.24,.90,.98,green_m)
                for dz in [-.129,.129]:
                    panel(g,x*1.025,d+dz,.022,.295,.90,trim,depth=.030)
                    panel(g,x*.97,d+dz*.94,.014,.31,.89,rubber,depth=.032)
                panel(g,x*1.02,d,.268,.285,.31,trim,depth=.045)
                if x>0:
                    for dz in [-.0605,.0605]:
                        door_leaf(g,x*.99,d+dz)
        if kind=='cab-front':cab(g,1);gangway(g,-1)
        elif kind=='cab-rear':cab(g,-1);gangway(g,1)
        else:gangway(g,1);gangway(g,-1)
        # Two detailed bogies, axle boxes, brake discs, suspension and conduits.
        for z in [-1.06,1.06]:
            g.box(0,.15,z,.56,.10,.45,plain,(58,67,73))
            for x in [-.27,.27]:
                for zz in [z-.14,z+.14]:
                    g.box(x*1.13,.15,zz,.055,.06,.06,plain,(78,95,101))
                g.beam((x,.18,z-.22),(x,.18,z+.22),.026,plain,(75,90,98))
        for z in [-.60,.60]:
            g.box(0,1.045,z,.41,.085,.43,plain,(147,163,169))
            for k in range(6):g.box(-.16+k*.064,1.09,z,.018,.008,.35,plain,(81,100,110))
        # Single-arm pantograph, contact slider at 1.25 above rail.
        g.box(0,1.03,0,.24,.035,.26,plain,(126,144,150))
        for x in [-.065,.065]:
            g.beam((x,1.045,-.20),(x,1.15,.18),.012,plain,(77,95,105))
            g.beam((x,1.15,.18),(x,1.25,-.06),.010,plain,(77,95,105))
        g.beam((-.23,1.25,-.06),(.23,1.25,-.06),.012,plain,(56,72,80))
        stretch(g);g.finish('transit-'+kind)

    # Four shared wheelsets, independent of the stationary bogie frame.
    # No longitudinal stretch: wheel cross-sections remain circular.
    for axle,raw_z in enumerate([-1.20,-.92,.92,1.20]):
        g=Geometry(a);z=raw_z*Z_SCALE
        g.cylinder(0,.09,z,.024,.54,plain,(69,77,82),axis='x',segments=16,smooth=True)
        for side in [-1,1]:
            x=side*.27
            g.cylinder(x,.09,z,.09,.060,plain,(54,63,70),axis='x',segments=24,smooth=True)
            g.cylinder(side*.307,.09,z,.060,.014,trim,axis='x',segments=24,smooth=True)
            for j in range(6):
                angle=j*math.tau/6
                g.cylinder(side*.319,.09+.042*math.cos(angle),z+.042*math.sin(angle),.006,.007,plain,(111,123,128),axis='x',segments=8,smooth=True)
            g.beam((side*.324,.149,z),(side*.324,.166,z),.007,plain,(170,175,168))
        g.finish('transit-wheel-'+str(axle))

    # Sliding platform-side leaves. Inset pane, gasket, rail, pocket and door seam.
    for part,offset in [('door-a',-.0605),('door-b',.0605)]:
        g=Geometry(a)
        for z in DOORS:
            x=-.357;door_leaf(g,x,z+offset)
            for dz in [-.056,.056]:panel(g,x-.015,z+offset+dz,.006,.32,.88,trim,depth=.012)
            panel(g,x-.024,z+offset,.014,.45,.49,trim,depth=.015)
        stretch(g);g.finish('transit-'+part)

    # Three-dimensional longitudinal seating: backs against side windows,
    # passengers face the aisle. Geometry is shared across all six coaches.
    g=Geometry(a)
    for side in [-1,1]:
        for z0 in [-.80,0,.80]:
            zz=z0*Z_SCALE
            g.box(side*.27,.39,zz,.155,.045,.70,seat)
            g.box(side*.325,.49,zz,.035,.20,.70,seat)
            for j in range(4):
                z=zz+(j-1.5)*.156;x=side*.25;idx=(j+int((z0+1)*10)+int(side))%len(coats)
                # Back/torso, seated thighs, shins, feet, neck, head, hair, nose.
                g.ellipsoid(x,.49,z,.050,.085,.053,coats[idx],sides=12,rings=6)
                g.box(side*.18,.407,z,.16,.050,.09,pants)
                g.box(side*.105,.345,z,.045,.115,.085,pants)
                g.box(side*.085,.289,z,.10,.030,.09,pants)
                g.cylinder(x,.571,z,.019,.035,skin[j%4],segments=12,smooth=True)
                g.ellipsoid(x,.615,z,.032,.042,.034,skin[j%4],sides=14,rings=7)
                g.ellipsoid(x+side*.004,.638,z,.034,.022,.035,hair[j%3],sides=12,rings=5)
                g.ellipsoid(x-side*.033,.615,z,.010,.010,.010,skin[j%4],sides=8,rings=4)
                for dz in [-.052,.052]:g.beam((x,.535,z+dz),(side*.14,.425,z+dz),.025,coats[idx],(255,255,255))
        for z in DOORS:
            zz=z*Z_SCALE
            g.cylinder(side*.17,.63,zz,.009,.68,trim,segments=10,smooth=True)
    for i,z0 in enumerate([-.40,.40]):
        z=z0*Z_SCALE;x=-.08 if i==0 else .08
        g.ellipsoid(x,.553,z,.050,.115,.054,coats[1+i],sides=12,rings=6)
        for dz in [-.026,.026]:g.box(x,.363,z+dz,.057,.18,.036,pants)
        g.ellipsoid(x,.72,z,.034,.044,.036,skin[1+i],sides=14,rings=7)
        g.ellipsoid(x,.744,z,.035,.022,.037,hair[i],sides=12,rings=5)
        g.beam((x,.62,z-.06),(-.17 if i==0 else .17,.72,z-.06),.024,coats[1+i],(255,255,255))
    # Floor, ceiling, end partitions and luminous diffusers remain behind glass.
    g.box(0,.96,0,.66,.02,4.20,wall)
    for x in [-.17,.17]:g.box(x,.944,0,.065,.012,4.10,led)
    for z in [-2.03,2.03]:g.box(0,.59,z,.61,.58,.025,wall)
    g.finish('transit-interior')
