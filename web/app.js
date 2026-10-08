import {createMeshViewer} from './viewer.js';
const $=id=>document.getElementById(id);
try {
 const viewer=createMeshViewer($('view'));
 const response=await fetch('hagane.wasm');if(!response.ok)throw Error('Build WASM first: ./scripts/build-web.sh');
 const {instance}=await WebAssembly.instantiate(await response.arrayBuffer(),{}),kernel=instance.exports;let mesh;
 window.haganeDemo={ready:false,setAngle:a=>viewer.setAngle(a),render:()=>viewer.render()};
 function generate(){const radius=Number($('radius').value);const preset=Number($('preset').value);const status=kernel.hagane_generate_preset(preset,radius,0.05);const text=new TextDecoder().decode(new Uint8Array(kernel.memory.buffer,kernel.hagane_output_ptr(),kernel.hagane_output_len()));mesh=JSON.parse(text);if(status)throw Error(mesh.error);viewer.setMesh(mesh);const count=mesh.positions.length/3;const names=['Through bore','Four through bores','L-profile extrusion','Hollow tube','Rotated four-bore part','Rounded profile extrusion'];const descriptions=['80 × 60 × 24 mm block','80 × 60 × 24 mm · four holes','Concave profile · polygon hole · skew extrusion','Ø60 mm outer · 24 mm height','Rigid B-rep placement · preserved exact bores','80 × 60 × 24 mm · exact line/arc boundary'];$('part-title').textContent=names[preset];$('part-description').textContent=descriptions[preset];$('radius').disabled=preset===2;$('radius-title').textContent=preset===3?'Inner radius':preset===5?'Corner radius':'Bore radius';$('radius-label').value=preset===2?'Fixed profile':((preset===1||preset===4)?radius*0.5:radius).toFixed(1)+' mm';$('topology').textContent=mesh.faces+' faces / '+mesh.edges+' edges';$('volume').textContent=mesh.volume.toLocaleString('en-US',{maximumFractionDigits:1})+' mm³';$('triangles').textContent=(count/3).toLocaleString('en-US');$('status').textContent='✓ Closed B-rep validated in Rust';window.haganeDemo.mesh=mesh;viewer.render();}

 const regenerate=()=>{try{generate();}catch(e){$('status').textContent=e.message;}};
 $('preset').addEventListener('change',regenerate);$('radius').addEventListener('input',regenerate);
 $('wire').addEventListener('change',()=>viewer.wireframe=$('wire').checked);$('spin').addEventListener('change',()=>viewer.autoRotate=$('spin').checked);$('reset').addEventListener('click',()=>viewer.reset());
 generate();window.haganeDemo.ready=true;
}catch(e){$('status').textContent=e.message;console.error(e);}
