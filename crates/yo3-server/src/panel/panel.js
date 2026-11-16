// The admin panel (#36): asks before a form marked data-confirm is sent. The only script the panel runs.
document.addEventListener("submit", (event) => {
  const question = event.target.dataset && event.target.dataset.confirm;
  if (question && !window.confirm(question)) event.preventDefault();
});
