/**
 * @name umi 的路由配置
 * @description 只支持 path,component,routes,redirect,wrappers,name,icon 的配置
 * @param path  path 只支持两种占位符配置，第一种是动态参数 :id 的形式，第二种是 * 通配符，通配符只能出现路由字符串的最后。
 * @param component 配置 location 和 path 匹配后用于渲染的 React 组件路径。可以是绝对路径，也可以是相对路径，如果是相对路径，会从 src/pages 开始找起。
 * @param routes 配置子路由，通常在需要为多个路径增加 layout 组件时使用。
 * @param redirect 配置路由跳转
 * @param wrappers 配置路由组件的包装组件，通过包装组件可以为当前的路由组件组合进更多的功能。 比如，可以用于路由级别的权限校验
 * @param name 配置路由的标题，默认读取国际化文件 menu.ts 中 menu.xxxx 的值，如配置 name 为 login，则读取 menu.ts 中 menu.login 的取值作为标题
 * @param icon 配置路由的图标，取值参考 https://ant.design/components/icon-cn， 注意去除风格后缀和大小写，如想要配置图标为 <StepBackwardOutlined /> 则取值应为 stepBackward 或 StepBackward，如想要配置图标为 <UserOutlined /> 则取值应为 user 或者 User
 * @doc https://umijs.org/docs/guides/routes
 */
export default [
  {
    path: '/user',
    layout: false,
    routes: [
      {
        path: '/user/login',
        name: 'login',
        component: './Login',
      },
      {
        path: '/user/forgot-password',
        name: 'forgot-password',
        component: '@/modules/password-reset/pages/ForgotPassword',
      },
    ],
  },
  {
    path: '/Production',
    name: 'production',
    icon: 'DesktopOutlined',
    routes: [
      {
        path: '/Production/estimate',
        name: 'estimate',
        access: 'canAccessEstimatePlan',
        component: './Production/Estimate',
      },
      {
        path: '/Production/apply',
        name: 'apply',
        access: 'canAccessApplyOrder',
        component: './Production/Apply',
      },
      {
        path: '/Production/purchase',
        name: 'purchase',
        routes: [
          {
            path: 'create',
            name: 'create',
            access: 'canAccessPurchaseProductionCreate',
            component: '../modules/purchase/production-request/pages/CreatePurchaseRequest',
          },
          {
            path: 'ledger',
            name: 'ledger',
            access: 'canAccessPurchaseProductionLedger',
            component: '../modules/purchase/production-request/pages/ProductionLedger/index',
          },
          // 260831 增量：从 /Procure 移到 /Production/purchase（与生产确认单业务紧邻）
          // 业务背景：发料申请单由生产计划员创建（在生产确认单之前），避免进入采购模块
          {
            path: 'materialApplication',
            name: 'materialApplication',
            access: 'canSubcontractMaterialApplication',
            component: '../modules/subcontract-application/pages/index',
          },
          // SP10 增量：生产确认单台账（委外到货生产确认草稿管理入口）
          {
            path: 'productionConfirmation',
            name: 'productionConfirmation',
            access: 'canProductionConfirmation',
            component: '@/modules/arrival-production-confirmation/pages/ProductionConfirmationList',
          },
        ],
      },
      {
        path: '/Production/receive',
        name: 'productionReceive',
        access: 'canAccessReceiveMate',
        component: './Production/Receive',
      },
      {
        path: '/Production/handleOrder',
        name: 'handleOrder',
        access: 'canAccessHandleOrder',
        component: './Production/WorkOrder',
      },
      {
        path: '/Production/lineSideWarehouse',
        name: 'lineSideWarehouse',
        access: 'canAccessLineSideWarehouse',
        component: './Production/LineSideWarehouse',
      },
      // 不再展示
      // {
      //   path: '/Production/lineSideNonConfirmWarehouse',
      //   name: 'lineSideNonConfirmWarehouse',
      //   access: 'canAccessLineSideNonConfirmWarehouse',
      //   component: './Production/LineSideNonConfirmWarehouse',
      // },
      {
        path: '/Production/enterpro',
        name: 'enterpro',
        access: 'canAccessProInspection',
        component: './Production/Enterpro',
      },
      {
        path: '/Production/inspectionReceive',
        name: 'inspectionReceive',
        access: 'canAccessInspectionReceive',
        component: './Production/InspectionReceive',
      },
      {
        path: '/Production/consum',
        name: 'adminconsum',
        access: 'canAccessAdminConsum',
        component: './Production/Consum',
      },
      {
        path: '/Production/exitapply',
        name: 'exitapply',
        access: 'canAccessReturnStock',
        component: './Production/Exitapply',
      },
      {
        path: '/Production/process',
        name: 'process',
        access: 'canAccessAdminProcess',
        component: './Production/Process',
      },
      {
        path: '/Production/UrgentRelease',
        name: 'UrgentRelease',
        access: 'canAccessUrgentRelease',
        component: './Production/UrgentRelease',
      },
      {
        path: '/Production/workingHours',
        name: 'workingHours',
        access: 'canAccessWorkingHours',
        component: './Production/WorkingHours',
      },
      {
        path: '/Production/dashboard',
        name: 'dashboard',
        access: 'canAccessProductionDashboard',
        component: './Production/Dashboard',
      },
    ],
  },
  {
    path: '/Procure',
    name: 'procure',
    icon: 'ShoppingCartOutlined',
    routes: [
      {
        path: '/Procure/purchaseLedger',
        name: 'purchaseLedger',
        access: 'canAccessProcurementLedger',
        component: '../modules/purchase/production-request/pages/ProcurementLedger',
      },
      {
        path: '/Procure/contract',
        name: 'contract',
        access: 'canAccessPurchaseContractLedger',
        component: '../modules/purchase/contract/pages/PurchaseContractLedger',
      },
      {
        path: '/Procure/paymentRequest',
        name: 'paymentRequest',
        access: 'canAccessPaymentRequestLedger',
        component: '../modules/payment/pages/PaymentRequestLedger',
      },
      {
        path: '/Procure/arrivalNotice',
        name: 'arrivalNotice',
        access: 'canAccessPurchaseArrivalNotice',
        component: '../modules/arrival-notification/pages/PurchaseArrivalNoticeLedger',
      },
      // {
      //   path: '/Procure/handOrder',
      //   name: 'handOrder',
      //   access: 'canAccessApplyHandle',
      //   component: './Procure/Order',
      // },
      // {
      //   path: '/Procure/arrival',
      //   name: 'arrival',
      //   access: 'canAccessArrivalNotice',
      //   component: './Procure/Arrival',
      // },
      {
        path: '/Procure/addVendor',
        name: 'addVendor',
        access: 'canAccessAdminVendor',
        component: './Procure/Admin',
      },
      {
        path: '/Procure/vendorInfo',
        name: 'vendorInfo',
        access: 'canAccessVendorInfo',
        component: './Procure/Vendor',
      },
      {
        path: '/Procure/afterSales',
        name: 'afterSales',
        access: 'canAccessArrivalNotice',
        component: './Procure/AfterSales',
      },
    ],
  },
  {
    path: '/Finance',
    name: 'finance',
    icon: 'DollarOutlined',
    routes: [
      {
        path: '/Finance/contract/ledger',
        name: 'financeContract',
        access: 'canAccessFinanceContractLedger',
        component: '../modules/purchase/contract/pages/FinanceContractLedger',
      },
      {
        path: '/Finance/payment/ledger',
        name: 'paymentLedger',
        access: 'canAccessPaymentLedger',
        component: '../modules/payment/pages/PaymentProcessLedger',
      },
    ],
  },
  {
    path: '/Supplier',
    name: 'supplier',
    icon: 'ToolOutlined',
    routes: [
      {
        path: '/Supplier/ledger',
        name: 'ledger',
        access: 'canAccessSupplierLedger',
        component: '../modules/supplier/pages/SupplierLedger',
      },
    ],
  },

  {
    path: '/Quality',
    name: 'quality',
    icon: 'ToolOutlined',
    routes: [
      {
        path: '/Quality/inspectionLedger',
        name: 'inspectionLedger',
        access: 'canAccessInspectionLedger',
        component: '../modules/inspection/pages/InspectionLedger',
      },
      {
        path: '/Quality/recmat',
        name: 'recmat',
        access: 'canAccessReceiveMaterial',
        component: './Quality/Recmat',
      },
      {
        path: '/Quality/testres',
        name: 'testres',
        access: 'canAccessInspectionMaterial',
        component: './Quality/Testres',
      },
      {
        path: '/Quality/review',
        name: 'review',
        access: 'canAccessInspectionReview',
        component: './Quality/Review',
      },
      {
        path: '/Quality/release',
        name: 'release',
        access: 'canAccessInspectionRelease',
        component: './Quality/Release',
      },
      {
        path: '/Quality/fillOrder',
        name: 'orders',
        access: 'canAccessFillOrder',
        component: './Quality/Orders',
      },
      {
        path: '/Quality/laboratory',
        name: 'laboratory',
        // 父路由不加 access: 由 3 个子路由各自的 access 字段独立控制 (D1 拆分激活)
        routes: [
          {
            path: '/Quality/laboratory/consumable',
            name: 'consumable',
            // D1 拆分激活: 耗材独立 subject (D1 业务边界独立)
            access: 'canAccessLabConsumable',
            component: './Quality/Labora/Consumable',
          },
          {
            path: '/Quality/laboratory/reagent',
            name: 'reagent',
            // D1 拆分激活: 试剂独立 subject (从父路由剥离, 字段名从 canAccessLaboraConsum 改名为 canAccessLabReagent)
            access: 'canAccessLabReagent',
            component: './Quality/Labora/Reagent',
          },
          {
            path: '/Quality/laboratory/strain',
            name: 'strain',
            // D1 拆分激活: 菌种独立 subject
            access: 'canAccessLabStrain',
            component: './Quality/Labora/Strain',
          },
        ],
      },
      {
        path: '/Quality/inspection-reagent',
        name: 'inspectionReagent',
        access: 'canAccessInspectionReagent',
        component: './Quality/InspectionReagent',
      },
      // 批记录
      // {
      //   path: '/Quality/checkOrder',
      //   name: 'checkOrder',
      //   access: 'canAccessCheckOrder',
      //   component: './Quality/Check',
      // },
      {
        path: '/Quality/checkWorkOrder',
        name: 'checkWorkOrder',
        access: 'canAccessWorkOrderReview',
        component: './Quality/CheckWorkOrder',
      },
      {
        path: '/Quality/device',
        name: 'device',
        access: 'canAccessDevice',
        routes: [
          {
            path: '/Quality/device/device',
            name: 'device',
            component: './Quality/Device/Device',
            routes: [
              {
                path: '/Quality/device/device/',
                component: './Quality/Device/Device/New',
              },
              {
                path: '/Quality/device/device/reviewed',
                component: './Quality/Device/Device/Reviewed',
              },
              {
                path: '/Quality/device/device/disposal',
                component: './Quality/Device/Device/Disposal',
              },
              {
                path: '/Quality/device/device/all',
                component: './Quality/Device/Device/All',
              },
            ],
          },
          {
            path: '/Quality/device/fixture',
            name: 'fixture',
            component: './Quality/Device/Fixture',
            routes: [
              {
                path: '/Quality/device/fixture/',
                component: './Quality/Device/Fixture/New',
              },
              {
                path: '/Quality/device/fixture/reviewed',
                component: './Quality/Device/Fixture/Reviewed',
              },
              {
                path: '/Quality/device/fixture/disposal',
                component: './Quality/Device/Fixture/Disposal',
              },
            ],
          },
          {
            path: '/Quality/device/measure',
            name: 'measure',
            component: './Quality/Device/Measure',
            routes: [
              {
                path: '/Quality/device/measure/',
                component: './Quality/Device/Measure/New',
              },
              {
                path: '/Quality/device/measure/reviewed',
                component: './Quality/Device/Measure/Reviewed',
              },
              {
                path: '/Quality/device/measure/disposal',
                component: './Quality/Device/Measure/Disposal',
              },
            ],
          },
          {
            path: '/Quality/device/measure-list',
            name: 'measure-list',
            component: './Quality/Device/MeasureList',
          },
        ],
      },
      {
        path: '/Quality/sip',
        name: 'adminSip',
        access: 'canAccessAdminSIP',
        component: './Quality/SIP',
      },
      {
        path: '/Quality/inspectionTpl',
        name: 'inspection-tpl',
        icon: 'ExperimentOutlined',
        access: 'canAccessInspectionTemplate',
        component: './Quality/InspectionTpl',
      },
      {
        path: '/Quality/controlledDoc',
        name: 'controlledDoc',
        access: 'canAccessControlledDoc',
        component: './Quality/ControlledDoc',
      },
      {
        path: '/Quality/sampleRetention',
        name: 'sampleRetention',
        // D6 序号4: 改名 canAccessSampleRetention → canAccessRetentionSample (D 决策)
        access: 'canAccessRetentionSample',
        routes: [
          {
            path: '/Quality/sampleRetention/list',
            name: 'list',
            component: './Quality/SampleRetention/List',
          },
          {
            path: '/Quality/sampleRetention/record',
            name: 'record',
            component: './Quality/SampleRetention/Record',
          },
          {
            path: '/Quality/sampleRetention/observation',
            name: 'observation',
            component: './Quality/SampleRetention/Observation',
          },
        ],
      },
    ],
  },
  {
    path: '/SubcontractWarehouse',
    name: 'subcontractWarehouse',
    icon: 'HomeOutlined',
    routes: [
      {
        path: '/SubcontractWarehouse/ledger',
        name: 'ledger',
        access: 'canSubcontractWarehouse',
        component: '../modules/subcontract-warehouse/pages/SubcontractInventoryLedgerPage',
      },
      {
        path: '/SubcontractWarehouse/check',
        name: 'check',
        access: 'canSubcontractWarehouse',
        component: '../modules/subcontract-warehouse/pages/SubcontractInventoryCheckPage',
      },
    ],
  },
  {
    path: '/Warehouse',
    name: 'warehouse',
    icon: 'HomeOutlined',
    routes: [
      {
        path: '/Warehouse/arrivalNotice',
        name: 'arrivalNotice',
        access: 'canAccessWarehouseArrivalNotice',
        component: '../modules/arrival-notification/pages/WarehouseArrivalNoticeLedger',
      },
      // {
      //   path: '/Warehouse/checkdoc',
      //   name: 'checkdoc',
      //   access: 'canAccessInitiateInspection',
      //   component: './Warehouse/Checkdoc',
      // },
      {
        path: '/Warehouse/packagesManagement',
        name: 'packagesManagement',
        access: 'canAccessEntryStock',
        component: './Warehouse/PackagesManagement',
      },
      {
        path: '/Warehouse/enterdoc',
        name: 'enterdoc',
        access: 'canAccessEntryStock',
        component: './Warehouse/Enterdoc',
      },
      {
        path: '/Warehouse/exportdoc',
        name: 'exportdoc',
        access: 'canAccessIssueStock',
        component: './Warehouse/Exportdoc',
      },
      {
        path: '/Warehouse/adminMate',
        name: 'adminmate',
        access: 'canAccessAdminMaterial',
        component: './Warehouse/Material',
      },
      // {
      //   path: '/Warehouse/reportform',
      //   name: 'reportform',
      //   access: 'canAccessReport',
      //   component: './Warehouse/Reportform',
      // },
      {
        path: '/Warehouse/conformedReportForm',
        name: 'conformedReportForm',
        access: 'canAccessConformedReportForm',
        component: './Warehouse/ConformedReportForm',
      },
      {
        path: '/Warehouse/nonconformedReportForm',
        name: 'nonconformedReportForm',
        access: 'canAccessNonconformedReportForm',
        component: './Warehouse/NonconformedReportForm',
      },
      {
        path: '/Warehouse/materialAuxiliaryReportForm',
        name: 'materialAuxiliaryReportForm',
        access: 'canAccessMaterialAuxiliaryReportForm',
        component: './Warehouse/MaterialAuxiliaryReportForm',
      },
      // 不再展示
      // {
      //   path: '/Warehouse/lineSideWarehouseReportForm',
      //   name: 'lineSideWarehouseReportForm',
      //   access: 'canAccessReport',
      //   component: './Warehouse/LineSideWarehouseReportForm',
      // },
      {
        path: '/Warehouse/adminStock',
        name: 'adminStock',
        access: 'canAccessAdminStock',
        component: './Warehouse/Admin',
      },
      {
        path: '/Warehouse/deliver-orders',
        name: 'deliverOrders',
        access: 'canAccessDeliverOrder',
        component: './Warehouse/DeliverOrders',
      },
      {
        path: '/Warehouse/delivery-notice-actions',
        name: 'deliveryNoticeActions',
        access: 'canAccessDeliveryNoticeAction',
        component: './Warehouse/DeliveryNoticeActions',
      },
      {
        path: '/Warehouse/deliver-notices',
        name: 'deliverNotices',
        access: 'canAccessDeliverNotice',
        component: './Warehouse/DeliverNotices',
      },
    ],
  },
  {
    path: '/R&D',
    name: 'R&D',
    icon: 'BulbOutlined',
    routes: [
      {
        path: '/R&D/task',
        name: 'task',
        access: 'canAccessTask',
        component: '@/modules/task/pages/Examine',
      },
      {
        path: '/R&D/print',
        name: 'print',
        access: 'canAccessPrint',
        component: './R&D/Print',
      },
      {
        path: '/R&D/requisition',
        name: 'requisition',
        access: 'canAccessRequisition',
        component: './R&D/Resexam',
      },
      {
        path: '/R&D/change',
        name: 'change',
        access: 'canAccessChangeRequest',
        component: '../modules/change-management/pages/ChangeLedger',
      },
      {
        path: '/R&D/deviation',
        name: 'deviation',
        access: 'canAccessDeviation',
        component: './R&D/Deviation',
      },
      {
        path: '/R&D/unacceptedProduct',
        name: 'unacceptedProduct',
        access: 'canAccessNonConformityReview',
        component: './R&D/UnacceptedProduct',
      },
    ],
  },
  {
    path: '/Other',
    name: 'other',
    icon: 'StarOutlined',
    routes: [
      {
        path: '/Other/addCheck',
        name: 'addCheck',
        component: './Other/AddCheck',
      },
      // {
      //   path: '/Other/addPurch',
      //   name: 'addPurch',
      //   access: 'canAccessCreateRootProcureRequisition',
      //   component: './Other/AddPurch',
      // },
      {
        path: '/Other/addBom',
        name: 'addBom',
        access: 'canAccessAdminUser',
        component: './Other/AddBom',
      },
      {
        path: '/Other/purchaseLedger',
        name: 'purchaseLedger',
        access: 'canAccessResearchPurchaseRequestLedger',
        component: '../modules/purchase/production-request/pages/ResearchPurchaseLedger/index',
      },
    ],
  },
  {
    path: '/research',
    name: 'research',
    icon: 'GlobalOutlined',
    routes: [
      {
        path: '/research/trialProApply',
        name: 'tryProApply',
        icon: 'FileUnknownOutlined',
        access: 'canAccessTrialProApply',
        component: './Word/Proapply',
      },
    ],
  },
  {
    path: '/udi',
    name: 'udi',
    icon: 'BarcodeOutlined',
    routes: [
      {
        path: '/udi/product',
        name: 'product',
        access: 'canAccessProduct',
        component: './Udi/ProductManagement',
      },
      {
        path: '/udi/company',
        name: 'company',
        icon: 'ContactsOutlined',
        access: 'canAccessCompany',
        component: './Udi/Company',
      },
      {
        path: '/udi/companyCode',
        name: 'companyCode',
        icon: 'ContactsOutlined',
        access: 'canAccessCompanyCode',
        component: './Udi/CompanyCode',
      },
      {
        path: '/udi/packaging',
        name: 'packaging',
        access: 'canAccessPackaging',
        component: './Udi/PackagingManagement',
      },
      {
        path: '/udi/packagingRelations',
        name: 'packagingRelations',
        access: 'canAccessRelations',
        component: './Udi/PackagingRelations',
      },
      {
        path: '/udi/udi',
        name: 'udi',
        access: 'canAccessUdiReportForm',
        component: './Udi/Udi',
      },
      {
        path: '/udi/tagPrint',
        name: 'tagPrint',
        access: 'canAccessUdiTagPrint',
        component: './Udi/TagPrint',
      },
      {
        path: '/udi/shapedSpongeRelations',
        name: 'shapedSpongeRelations',
        access: 'canAccessUdiShapedSpongeRelations',
        component: './Udi/ShapedSpongeRelations',
      },
      {
        path: '/udi/shapedSponge',
        name: 'shapedSponge',
        access: 'canAccessShapedSponge',
        component: './Udi/ShapedSponge',
      },
    ],
  },
  {
    path: '/workOrderTemplate',
    name: 'workOrder',
    icon: 'ContainerOutlined',
    access: 'canAccessWorkOrderTemplate',
    component: './Records/WorkOrder',
  },
  {
    path: '/ai',
    name: 'ai',
    icon: 'BulbOutlined',
    access: 'canAccessAI',
    routes: [
      {
        path: '/ai/knowledge',
        name: 'knowledge',
        component: './AI/Knowledge',
      },
      {
        path: '/ai/editor',
        name: 'editor',
        access: 'canAccessAdminUser',
        component: './AI/Editor',
      },
    ],
  },
  // {
  //   path: '/ai',
  //   name: 'ai',
  //   icon: 'RedditOutlined',
  //   component: './Ai',
  // },
  {
    path: '/accMate',
    name: 'accMate',
    icon: 'FileDoneOutlined',
    access: 'canAccessAcceptMaterial',
    component: './Word/Accmate',
  },
  {
    path: '/operation',
    name: 'operation',
    icon: 'GlobalOutlined',
    routes: [
      {
        path: '/operation/users',
        name: 'users',
        icon: 'UserOutlined',
        access: 'canAccessAdminUser',
        component: '../modules/users/pages/UserLedger',
      },
      {
        path: '/operation/user-roles',
        name: 'userRoles',
        icon: 'UserSwitchOutlined',
        access: 'canAccessUserRole',
        component: '../modules/users/pages/UserRolesLedger',
      },
      {
        path: '/operation/department',
        name: 'department',
        icon: 'TeamOutlined',
        access: 'canAccessDepartmentList',
        component: '../modules/cost-department/pages/CostDepartmentLedger',
      },
      {
        path: '/operation/project',
        name: 'project',
        icon: 'ProjectOutlined',
        access: 'canAccessProjectManage',
        component: '../modules/project/pages/ProjectLedger',
      },
      {
        path: '/operation/auditLog',
        name: 'auditLog',
        icon: 'FileTextOutlined',
        access: 'canAccessAuditLog',
        component: '../modules/audit-log/pages/AuditLogLedger',
      },
      {
        path: '/operation/permissions',
        name: 'permissions',
        icon: 'SafetyCertificateOutlined',
        access: 'canAccessPermissionAdmin',
        component: '../modules/permissions/pages/PermissionsAdmin',
      },
      {
        path: '/operation/form',
        name: 'forms',
        icon: 'UserOutlined',
        access: 'canAccessForm',
        component: './Operation/Form',
      },
      {
        path: '/operation/tools',
        name: 'tools',
        icon: 'ToolOutlined',
        access: 'canAccessTools',
        component: './SystemTools/Tools',
      },
    ],
  },
  {
    path: '/records',
    name: 'records',
    component: './Records',
    layout: false,
  },
  {
    path: '/woTemplate',
    name: 'woTemplate',
    component: './Records/WOTemp',
    layout: false,
  },
  {
    path: '/user/account',
    component: './Users/Account',
  },
  {
    path: '/env',
    component: './SystemTools/SwitchEnv',
  },
  {
    path: '/welcome',
    component: './Users/Welcome',
  },
  {
    path: '/gq',
    component: './GQ',
    layout: false,
  },
  {
    path: '/',
    redirect: './welcome',
  },
  {
    path: '*',
    layout: false,
    component: './404',
  },
];
