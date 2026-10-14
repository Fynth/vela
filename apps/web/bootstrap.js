import init from "/pkg/vela_web.js";

init().catch(() => {
  document.getElementById("main").textContent = "Vela could not start. Please refresh the page after checking your connection.";
});
