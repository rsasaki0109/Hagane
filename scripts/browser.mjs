import { chromium } from 'playwright';
import { createServer } from 'node:http';
import { readFile, mkdir } from 'node:fs/promises';
import { existsSync } from 'node:fs';
import { spawnSync } from 'node:child_process';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import assert from 'node:assert/strict';
const root=new URL('../web/',import.meta.url);
const types={'/':'text/html','/index.html':'text/html','/app.js':'text/javascript','/style.css':'text/css','/hagane.wasm':'application/wasm'};
const server=createServer(async(req,res)=>{try{const path=new URL(req.url,'http://localhost').pathname;if(!types[path]){res.writeHead(404);res.end();return;}const data=await readFile(new URL(path==='/'?'index.html':path.slice(1),root));res.writeHead(200,{'Content-Type':types[path]});res.end(data);}catch{res.writeHead(500);res.end('Build the WASM module first.');}});
await new Promise(resolve=>server.listen(0,'127.0.0.1',resolve));
let browser;
try {
 const executable=process.env.HAGANE_CHROMIUM ?? (existsSync('/usr/bin/chromium')?'/usr/bin/chromium':undefined);
 browser=await chromium.launch({executablePath:executable,headless:true,args:['--enable-unsafe-swiftshader','--use-gl=angle','--use-angle=swiftshader']});
 const page=await browser.newPage({viewport:{width:1440,height:900},deviceScaleFactor:1});const errors=[];page.on('pageerror',e=>errors.push(e.message));
 await page.goto(`http://127.0.0.1:${server.address().port}/`);await page.waitForFunction(()=>window.haganeDemo?.ready);
 const volume=await page.evaluate(()=>window.haganeDemo.mesh.volume);assert.ok(Math.abs(volume-(115200-Math.PI*196*24))<1e-8);
 const initial=await page.locator('canvas').screenshot();await page.locator('canvas').focus();await page.keyboard.press('ArrowRight');assert.notDeepEqual(await page.locator('canvas').screenshot(),initial);
 const canvas=await page.locator('canvas').boundingBox();await page.mouse.move(canvas.x+canvas.width/2,canvas.y+canvas.height/2);await page.mouse.down();await page.mouse.move(canvas.x+canvas.width/2+80,canvas.y+canvas.height/2+20,{steps:6});await page.mouse.up();const orbit=await page.locator('canvas').screenshot();assert.notDeepEqual(orbit,initial);
 await page.mouse.wheel(0,150);await page.waitForTimeout(100);assert.notDeepEqual(await page.locator('canvas').screenshot(),orbit);
 await page.locator('#radius').evaluate(e=>{e.value=24;e.dispatchEvent(new Event('input'));});assert.ok(Math.abs(await page.evaluate(()=>window.haganeDemo.mesh.volume)-(115200-Math.PI*24**2*24))<1e-8);assert.match(await page.locator('#radius-label').textContent(),/24.0/);
 const solid=await page.locator('canvas').screenshot();await page.locator('#wire').check();assert.notDeepEqual(await page.locator('canvas').screenshot(),solid);await page.locator('#wire').uncheck();
 await page.locator('#radius').evaluate(e=>{e.value=14;e.dispatchEvent(new Event('input'));});await page.locator('#reset').click();
 assert.equal(await page.evaluate(()=>document.getElementById('view').getContext('webgl').getError()),0);
 assert.deepEqual(errors,[]);
 if(process.argv.includes('--capture')){
  const frames=join(tmpdir(),'hagane-demo-frames');await mkdir(frames,{recursive:true});
  for(let i=0;i<48;i++){await page.evaluate(a=>window.haganeDemo.setAngle(a),0.65+2*Math.PI*i/48);await page.screenshot({path:join(frames,`frame-${String(i).padStart(3,'0')}.png`)});}
  const output=new URL('../docs/demo.gif',import.meta.url).pathname;
  const result=spawnSync('ffmpeg',['-y','-framerate','12','-i',join(frames,'frame-%03d.png'),'-filter_complex','[0:v]scale=1008:-1:flags=lanczos,split[a][b];[a]palettegen=stats_mode=diff[p];[b][p]paletteuse=dither=bayer:bayer_scale=3','-loop','0',output],{encoding:'utf8'});
  if(result.status!==0)throw Error(result.stderr);console.log('Recorded actual WASM/WebGL demo to docs/demo.gif (48 frames).');
 }
 await page.setViewportSize({width:390,height:844});await page.waitForTimeout(100);assert.equal(await page.evaluate(()=>document.getElementById('view').getContext('webgl').getError()),0);assert.deepEqual(errors,[]);
 console.log('Browser: WASM generation, radius, keyboard/drag orbit, wheel zoom, wireframe, reset, responsive rendering passed.');
} finally {await browser?.close();server.close();}
