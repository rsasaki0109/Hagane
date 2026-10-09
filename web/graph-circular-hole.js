import {createMeshViewer} from './viewer.js';
const $=id=>document.getElementById(id);
try{
 const viewer=createMeshViewer($('view')),response=await fetch('hagane.wasm');if(!response.ok)throw Error('Build WASM first: ./scripts/build-web.sh');
 const {instance}=await WebAssembly.instantiate(await response.arrayBuffer(),{}),k=instance.exports;window.haganeGraphCircularHole={ready:false,data:null};
 function generate(){
  const values=['width','depth','height','bulge','surface-error'].map(id=>$(id).valueAsNumber);values.push($('angle').valueAsNumber*Math.PI/180,...['tx','ty','tz','u-min','u-max','v-min','v-max','circle-x','circle-y','circle-radius'].map(id=>$(id).valueAsNumber));
  k.hagane_graph_circular_hole_begin();for(const value of values)if(k.hagane_graph_circular_hole_push(value))throw Error('Circle path transport rejected nonfinite input.');const status=k.hagane_graph_circular_hole_finish(),data=JSON.parse(new TextDecoder().decode(new Uint8Array(k.memory.buffer,k.hagane_output_ptr(),k.hagane_output_len())));if(status)throw Error(data.error);
  const center=[0,1,2].map(axis=>{let lo=Infinity,hi=-Infinity;for(let i=axis;i<data.positions.length;i+=3){lo=Math.min(lo,data.positions[i]);hi=Math.max(hi,data.positions[i]);}return(lo+hi)/2;}),segments=[];
  for(const samples of data.boundary_samples)for(let i=3;i<samples.length;i+=3)segments.push([samples.slice(i-3,i),samples.slice(i,i+3)]);
  viewer.setMesh({positions:data.positions.map((x,i)=>x-center[i%3]),normals:data.normals});viewer.setSegments(segments.map(segment=>segment.map(point=>point.map((x,i)=>x-center[i]))));window.haganeGraphCircularHole.data=data;
  $('surface-error-label').value=data.error.toFixed(3);$('volume').textContent=data.volume.toFixed(3)+' mm³';$('centroid').textContent=data.mass_properties.centroid.map(x=>x.toFixed(4)).join(', ');$('inertia').textContent=data.inertia_properties?data.inertia_properties.inertia.map(row=>row.map(x=>x.toExponential(5)).join('  ')).join('\n'):'Unavailable: '+data.inertia_error;$('faces').textContent=String(data.brep.faces);$('edges').textContent=String(data.brep.edges);$('triangles').textContent=String(data.mesh.triangles.length);$('surface-bound').textContent=data.error_bounds.reduce((a,b)=>Math.max(a,b),0).toExponential(3);$('genus').textContent=String(data.genus);$('cap-wires').textContent=data.brep.wire_edges.slice(0,2).map(w=>w.length).join(' / ');$('removed-volume').textContent=data.removed_volume.toFixed(3);$('status').textContent='✓ Closed circular-bore B-rep validated · exact rational trim · bounded display';
 }
 const update=()=>{try{generate();}catch(e){$('status').textContent='Circular bore rejected: '+e.message+' Accepted solid is preserved.';}};
 $('apply').addEventListener('click',update);$('surface-error').addEventListener('input',update);$('wire').addEventListener('change',()=>viewer.wireframe=$('wire').checked);$('reset').addEventListener('click',()=>viewer.reset());generate();window.haganeGraphCircularHole.ready=true;
}catch(e){$('status').textContent=e.message;console.error(e);}
