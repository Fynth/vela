// Static design artifact verification; does not open the live app or a user profile.
import {createRequire} from 'node:module';
import {readFile,writeFile} from 'node:fs/promises';
import {fileURLToPath,pathToFileURL} from 'node:url';
import {dirname,join} from 'node:path';
const here=dirname(fileURLToPath(import.meta.url));
const require=createRequire(new URL('../../../../tests/browser/package.json',import.meta.url));
const {chromium}=require('@playwright/test');
const browser=await chromium.launch({headless:true});
const evidence=[];
try {
 for(const [name,width,height,theme,view] of [
  ['desktop-light',1440,1100,'light','home'],['desktop-dark',1440,1100,'dark','home'],
  ['phone-light',375,812,'light','home'],['phone-dark',375,812,'dark','home'],
  ['album-light',1440,1100,'light','album'],['narrow-light',320,700,'light','home']
 ]){
  const page=await browser.newPage({viewport:{width,height},deviceScaleFactor:1});
  const errors=[]; page.on('pageerror',e=>errors.push(e.message));
  await page.goto(pathToFileURL(join(here,'vela-atlas-board.html')).href);
  await page.evaluate(()=>document.fonts.ready);
  if(theme==='dark') await page.locator('#theme-toggle').click();
  if(view==='album') await page.locator('#view-toggle').click();
  const state=await page.evaluate(()=>({width:innerWidth,height:innerHeight,scrollWidth:document.documentElement.scrollWidth,theme:document.documentElement.dataset.theme,view:document.body.dataset.view,fonts:[document.fonts.check('16px Golos'),document.fonts.check('36px Editorial')],audioElements:document.querySelectorAll('audio,iframe').length,player:document.querySelector('.player').getBoundingClientRect().toJSON(),artworks:document.querySelectorAll('[data-art] svg').length}));
  if(state.scrollWidth>width || state.fonts.some(v=>!v) || errors.length || state.theme!==theme || state.view!==view || state.audioElements)throw Error(`Design board verification failed: ${name} ${JSON.stringify({state,errors})}`);
  await page.screenshot({path:join(here,`atlas-${name}.png`),fullPage:true});
  evidence.push({name,...state,errors}); await page.close();
 }
 const tokens=JSON.parse(await readFile(join(here,'tokens.json'),'utf8'));
 const linear=v=>{v/=255;return v<=.04045?v/12.92:((v+.055)/1.055)**2.4};
 const luminance=hex=>{const rgb=hex.slice(1).match(/../g).map(v=>linear(parseInt(v,16)));return rgb[0]*.2126+rgb[1]*.7152+rgb[2]*.0722};
 const contrast=(a,b)=>{const x=luminance(a),y=luminance(b);return (Math.max(x,y)+.05)/(Math.min(x,y)+.05)};
 const ratios=[];
 for(const theme of ['light','dark'])for(const [text,bg]of [['text','canvas'],['text','surface'],['muted','canvas'],['muted','surface'],['onAccent','accent'],['link','canvas']]){
  const ratio=contrast(tokens[theme][text],tokens[theme][bg]);ratios.push({theme,text,bg,ratio:+ratio.toFixed(2),pass:ratio>=4.5});
 }
 await writeFile(join(here,'render-evidence.json'),JSON.stringify({scope:'Static design board only; not application E2E or full WCAG certification',captures:evidence,contrastPairs:ratios},null,2)+'\n');
 if(ratios.some(p=>!p.pass))throw Error('Proposed normal-text contrast pair failed');
 console.log(`Rendered ${evidence.length} design views; verified ${ratios.length} normal-text contrast pairs; no page errors or horizontal document overflow.`);
}finally{await browser.close();}
