import {createMeshViewer} from './viewer.js';
const $=id=>document.getElementById(id);
try{
 const viewer=createMeshViewer($('view'),{doubleSided:true}),response=await fetch('hagane.wasm');if(!response.ok)throw Error('Build WASM first: ./scripts/build-web.sh');
 const {instance}=await WebAssembly.instantiate(await response.arrayBuffer(),{}),k=instance.exports;window.haganeSurfaceEdge={ready:false,data:null};
 function generate(){
  const height=Number($('height').value),weight=Number($('weight').value),error=Number($('edge-error').value),start=['u-start','v-start'].map(id=>Number($(id).value)),end=['u-end','v-end'].map(id=>Number($(id).value));
  const status=k.hagane_generate_surface_edge(height,weight,...start,...end,error,Number($('crease').checked)),data=JSON.parse(new TextDecoder().decode(new Uint8Array(k.memory.buffer,k.hagane_output_ptr(),k.hagane_output_len())));if(status)throw Error(data.error);
  viewer.setMesh(data);const segments=[];for(let i=3;i<data.samples.length;i+=3)segments.push([data.samples.slice(i-3,i),data.samples.slice(i,i+3)]);const n=Math.hypot(...data.tangent);segments.push([data.point,data.point.map((x,i)=>x+data.tangent[i]*14/n)]);viewer.setSegments(segments);window.haganeSurfaceEdge.data=data;
  $('height-label').value=height.toFixed(0);$('weight-label').value=weight.toFixed(2);$('edge-error-label').value=error.toFixed(3);$('degree').textContent=String(data.curve.degree);$('span-count').textContent=String(data.span_ranges.length);$('segments').textContent=String(data.error_bounds.length);$('edge-bound').textContent=Math.max(...data.error_bounds).toExponential(3);$('status').textContent='✓ Exact B-rep edge validated · affine UV pcurve attached';
 }
 const update=()=>{try{generate();}catch(e){$('status').textContent=e.message;}};$('apply').addEventListener('click',update);for(const id of ['height','weight','edge-error'])$(id).addEventListener('input',update);$('crease').addEventListener('change',update);$('wire').addEventListener('change',()=>viewer.wireframe=$('wire').checked);$('reset').addEventListener('click',()=>viewer.reset());generate();window.haganeSurfaceEdge.ready=true;
}catch(e){$('status').textContent=e.message;console.error(e);}
