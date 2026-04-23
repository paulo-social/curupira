import { AlertTriangle, Siren } from 'lucide-react'

export function AlertsList({ alerts }) {
  return (
    <section className="rounded-3xl border border-white/10 bg-black/20 p-6 shadow-glow">
      <div className="mb-5 flex items-center justify-between">
        <div>
          <p className="text-sm uppercase tracking-[0.3em] text-forest-300/70">Alertas Recentes</p>
          <h2 className="mt-2 font-display text-2xl text-white">Histórico operacional</h2>
        </div>
        <Siren className="text-ember-500" />
      </div>

      <div className="space-y-3">
        {alerts.length === 0 ? (
          <div className="rounded-2xl border border-dashed border-white/10 p-4 text-white/60">
            Nenhum alerta crítico registrado ainda.
          </div>
        ) : (
          alerts.map((alert) => (
            <article
              key={alert.id}
              className="flex items-center justify-between rounded-2xl border border-white/10 bg-white/[0.04] px-4 py-3"
            >
              <div className="flex items-center gap-3">
                <div className="rounded-xl bg-ember-500/15 p-2 text-ember-500">
                  <AlertTriangle size={18} />
                </div>
                <div>
                  <p className="font-semibold capitalize text-white">{alert.tipo_evento}</p>
                  <p className="text-sm text-white/55">
                    {new Date(alert.timestamp).toLocaleString('pt-BR')}
                  </p>
                </div>
              </div>
              <div className="rounded-full bg-white/10 px-3 py-1 text-sm font-bold text-white">
                {Math.round(alert.confianca)}%
              </div>
            </article>
          ))
        )}
      </div>
    </section>
  )
}

