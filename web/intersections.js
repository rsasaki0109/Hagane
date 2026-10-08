import {createMeshViewer} from './viewer.js';
const $=id=>document.getElementById(id);
try{
 const viewer=createMeshViewer($('view'),{doubleSided:true});
 const response=await fetch('hagane.wasm');if(!response.ok)throw Error('Build WASM first: ./scripts/build-web.sh');const {instance}=await WebAssembly.instantiate(await response.arrayBuffer(),{}),k=instance.exports;
 window.haganeIntersections={ready:false,data:null};
 function update(){try{
  const segmentEllipse=$('scope').value==='segmentEllipsePlane';const halfEllipse=$('scope').value==='halfEllipsePlane';const chordEllipse=segmentEllipse||halfEllipse;const fullEllipse=$('scope').value==='ellipsePlane';const elliptical=fullEllipse||chordEllipse;if(fullEllipse)$('mode').value='0';$('mode').options[1].disabled=fullEllipse;$('mode').options[2].disabled=fullEllipse;const harmonic=$('scope').value.startsWith('band');$('mode').options[3].disabled=!harmonic;if(!harmonic&&$('mode').value==='3')$('mode').value='0';
  for(const key of ['generator','reverse','ellipse'])document.querySelector('#probe option[value="'+key+'"]').disabled=key==='ellipse'?elliptical:fullEllipse;
  const mode=Number($('mode').value),offset=Number($('offset').value);
  const scope=$('scope').value;$('offset-title').textContent=elliptical?'In-plane offset':mode===3?'Plane offset':'Radial offset';
  const status=segmentEllipse?k.hagane_ellipse_segment_planar_demo(Math.PI/2,mode,offset,0):halfEllipse?k.hagane_half_ellipse_planar_demo(mode,offset,0):elliptical?k.hagane_ellipse_planar_demo(offset,0):harmonic?k.hagane_harmonic_face_intersections_demo(Number(scope.slice(4)),mode,offset,0):scope==='surface'?k.hagane_extrusion_intersections_demo(mode,offset,0):k.hagane_circular_face_intersections_demo(Number(scope),mode,offset,0);
  const data=JSON.parse(new TextDecoder().decode(new Uint8Array(k.memory.buffer,k.hagane_output_ptr(),k.hagane_output_len())));if(status)throw Error(data.error);
  window.haganeIntersections.data=data;viewer.setMesh(data.display_mesh??data.mesh);
  $('part-title').textContent=segmentEllipse?'90° ellipse arc and chord':halfEllipse?'Half ellipse and diameter':elliptical?'Planar ellipse cap':harmonic?(scope==='band0'?'Below ellipse cut':'Above ellipse cut'):scope==='surface'?'Skew circular extrusion':scope==='0'?'Upper semicircular face':'Lower semicircular face';
  $('intro-title').innerHTML=elliptical?'A line.<br>A planar cap.<br>Exact clipping.':'A line.<br>A curved wall.<br>Exact contact.';
  $('intro-description').innerHTML=chordEllipse?'Clip against the exact arc and straight chord.<br>Cyan marks the line and resolved hits.':elliptical?'Clip against the exact ellipse boundary.<br>Cyan marks the line and resolved hits.':'Intersect a translated circular surface.<br>Cyan marks the line and resolved hits.';
  $('query-title').textContent=elliptical?'LINE / PLANAR TRIM':'LINE / SKEW WALL';$('mode').options[0].textContent=elliptical?'Across ellipse boundary':'Across circular wall';$('mode').options[1].textContent=chordEllipse?(segmentEllipse?'Chord → arc':'Diameter → arc'):'Along extrusion';$('mode').options[2].textContent=chordEllipse?(segmentEllipse?'Arc → chord':'Arc → diameter'):'Reverse extrusion';document.querySelector('#probe option[value="generator"]').textContent=chordEllipse?(segmentEllipse?'Chord → arc':'Diameter → arc'):'Generator overlap';document.querySelector('#probe option[value="reverse"]').textContent=chordEllipse?(segmentEllipse?'Arc → chord':'Arc → diameter'):'Reverse generator';
  $('part-description').textContent=segmentEllipse?'90° elliptic segment · oblique planar cap':halfEllipse?'Half ellipse 49.5 × 24 mm · straight diameter':elliptical?'Ellipse 49.5 × 48 mm · oblique planar cap':'Ø48 mm · 24 mm normal span · 12 / −6 mm tangential displacement';
  const {intersection:hit,line}=data;
  const at=t=>line.anchor.map((v,i)=>v+line.direction[i]*t);
  const segments=(hit.endpoints??hit.hits).map(p=>[p.point,p.point.map((v,i)=>v+p.normal[i]*14)]);
  segments.push(mode===3?[at(-1.2),at(1.2)]:(elliptical||mode===0)?[at(0),at(40)]:[at(-0.2),at(1.2)]);viewer.setSegments(segments);
  $('kind').textContent=hit.kind==='points'?hit.hits[0].contact:hit.kind==='coincident'?'generator overlap':'empty';
  $('hits').textContent=String(hit.hits.length);$('parameters').textContent=(hit.range??hit.hits.map(p=>p.parameter)).map(t=>t.toFixed(3)).join(' / ')||'—';
  $('volume').textContent=data.mesh.volume.toLocaleString('en-US',{maximumFractionDigits:1})+' mm³';$('offset-label').value=offset.toFixed(1)+' mm';$('status').textContent=scope==='surface'?'✓ Line and surface agreement checked in Rust':'✓ Face trims and original edge parameters checked in Rust';
 }catch(e){window.haganeIntersections.data=null;viewer.setSegments([]);$('kind').textContent='unresolved';$('hits').textContent='—';$('parameters').textContent='—';$('status').textContent=e.message;}}
 $('probe').addEventListener('change',()=>{const p={crossing:[0,14],tangent:[0,24],empty:[0,30],generator:[1,0],reverse:[2,0],ellipse:[3,0]}[$('probe').value];if(p){if($('scope').value==='segmentEllipsePlane'&&$('probe').value==='crossing')p[1]=20;if($('probe').value==='ellipse')$('scope').value='band0';$('mode').value=p[0];$('offset').value=p[1];update();}});
 for(const id of ['scope','mode','offset'])$(id).addEventListener(id==='offset'?'input':'change',()=>{if(id==='scope'&&['ellipsePlane','halfEllipsePlane','segmentEllipsePlane'].includes($('scope').value)){$('mode').value='0';$('offset').value=$('scope').value==='segmentEllipsePlane'?'20':'14';}$('probe').value='custom';update();});
 $('wire').addEventListener('change',()=>viewer.wireframe=$('wire').checked);$('spin').addEventListener('change',()=>viewer.autoRotate=$('spin').checked);$('reset').addEventListener('click',()=>viewer.reset());update();window.haganeIntersections.ready=true;
}catch(e){$('status').textContent=e.message;console.error(e);}
