<template>
  <UApp>
    <UContainer class="py-8">
      <UCard v-if="!loggedIn" class="max-w-md mx-auto">
        <template #header>
          <h1 class="text-xl font-bold">Backuper</h1>
        </template>
        <p class="text-gray-600 mb-4">请输入 API Token 登录</p>
        <UInput
          v-model="tokenInput"
          type="password"
          placeholder="API Token"
          class="w-full"
          @keyup.enter="login"
        />
        <UButton class="mt-4 w-full" :loading="loggingIn" @click="login">
          登录
        </UButton>
        <p v-if="loginError" class="mt-2 text-red-500 text-sm">登录失败</p>
      </UCard>

      <div v-else>
        <div class="flex items-center justify-between mb-6">
          <h1 class="text-2xl font-bold">Backuper Dashboard</h1>
          <UButton color="gray" size="sm" @click="logout">退出</UButton>
        </div>

        <div class="grid grid-cols-1 md:grid-cols-3 gap-4 mb-6">
          <UCard>
            <div class="text-sm text-gray-500">规则数</div>
            <div class="text-2xl font-bold">{{ status?.rules_count ?? 0 }}</div>
          </UCard>
          <UCard>
            <div class="text-sm text-gray-500">存储后端</div>
            <div class="text-2xl font-bold">{{ status?.storages_count ?? 0 }}</div>
          </UCard>
          <UCard>
            <div class="text-sm text-gray-500">通知渠道</div>
            <div class="text-2xl font-bold">{{ status?.notifiers_count ?? 0 }}</div>
          </UCard>
        </div>

        <UCard class="mb-6">
          <template #header>
            <h2 class="text-lg font-semibold">规则</h2>
          </template>
          <ul v-if="status?.rules?.length" class="space-y-2">
            <li
              v-for="rule in status.rules"
              :key="rule.id"
              class="flex items-center justify-between p-2 bg-gray-50 rounded"
            >
              <div>
                <div class="font-medium">{{ rule.id }}</div>
                <div class="text-xs text-gray-500">{{ rule.schedule }}</div>
              </div>
              <UButton size="xs" :loading="running === rule.id" @click="runRule(rule.id)">
                立即运行
              </UButton>
            </li>
          </ul>
          <p v-else class="text-gray-500">暂无规则</p>
        </UCard>

        <UCard>
          <template #header>
            <h2 class="text-lg font-semibold">最近任务</h2>
          </template>
          <UTable :rows="recentJobs" :columns="jobColumns" />
        </UCard>
      </div>
    </UContainer>
  </UApp>
</template>

<script setup lang="ts">
const token = useState<string | null>('token', () => null)
const tokenInput = ref('')
const loggingIn = ref(false)
const loginError = ref(false)
const running = ref<string | null>(null)

const loggedIn = computed(() => token.value !== null)

interface Rule {
  id: string
  schedule: string
}

interface Job {
  id: number
  rule_id: string
  status: string
  started_at: string
  finished_at?: string
  archive_key?: string
  error_message?: string
}

interface Status {
  rules_count: number
  storages_count: number
  notifiers_count: number
  rules: Rule[]
  recent_jobs: Job[]
}

const { data: status, refresh, pending } = await useFetch<Status>('/api/status', {
  headers: computed(() => (token.value ? { Authorization: `Bearer ${token.value}` } : {})),
  server: false,
})

watch(pending, (isPending) => {
  if (!isPending && !status.value && token.value) {
    token.value = null
  }
})

async function login() {
  loggingIn.value = true
  loginError.value = false
  try {
    const res = await $fetch<{ ok: boolean }>('/api/login', {
      method: 'POST',
      body: { token: tokenInput.value },
    })
    if (res.ok) {
      token.value = tokenInput.value
      await refresh()
    } else {
      loginError.value = true
    }
  } catch {
    loginError.value = true
  } finally {
    loggingIn.value = false
  }
}

async function logout() {
  token.value = null
}

async function runRule(ruleId: string) {
  running.value = ruleId
  try {
    await $fetch(`/api/run/${ruleId}`, {
      method: 'POST',
      headers: token.value ? { Authorization: `Bearer ${token.value}` } : undefined,
    })
    await refresh()
  } finally {
    running.value = null
  }
}

const recentJobs = computed(() =>
  (status.value?.recent_jobs ?? []).map((j: Job) => ({
    id: j.id,
    rule_id: j.rule_id,
    status: j.status,
    started_at: j.started_at,
    finished_at: j.finished_at ?? '-',
    archive_key: j.archive_key ?? '-',
    error_message: j.error_message ?? '-',
  }))
)

const jobColumns = [
  { key: 'id', label: 'ID' },
  { key: 'rule_id', label: '规则' },
  { key: 'status', label: '状态' },
  { key: 'started_at', label: '开始时间' },
  { key: 'finished_at', label: '结束时间' },
]
</script>
