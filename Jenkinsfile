#!/usr/bin/env groovy

pipeline {
    agent {
        docker {
            image 'yeanwang/x-kernel-builder:v1.0'
            args '-v /var/run/docker.sock:/var/run/docker.sock --privileged -u root:root'        }
    }

    environment {
        CI = 'true'
        TEST_HARNESS_REPO = 'https://gitee.com/openkylin/starry-test-harness'
        TEST_HARNESS_BRANCH = 'master'
    }

    stages {
        stage('Parallel Architecture Verification') {
            failFast true
            parallel {
                stage('Architecture: aarch64') {
                    steps {
                        script { executeBuildAndTest('aarch64') }
                    }
                }
                stage('Architecture: x86_64') {
                    steps {
                        script { executeBuildAndTest('x86_64') }
                    }
                }
            }
        }
    }

    post {
        always {
            archiveArtifacts artifacts: '**/artifacts/**/*', allowEmptyArchive: true
            archiveArtifacts artifacts: '**/logs/**/*', allowEmptyArchive: true
            cleanWs()
        }
        success {
            updateGiteeCommitStatus state: 'success', context: 'ci/jenkins'
        }
        unsuccessful {
            updateGiteeCommitStatus state: 'failed', context: 'ci/jenkins'
        }
    }
}

def executeBuildAndTest(arch) {
    ws("${WORKSPACE}/${arch}") {
        echo "Verifying architecture: ${arch}"
        
        checkout scm
        sh "git config --global --add safe.directory ${pwd()}"
        dir('test-harness') {
            git branch: "${env.TEST_HARNESS_BRANCH}", 
                url: "${env.TEST_HARNESS_REPO}",
                credentialsId: 'gitee-my-token'

            sh "git config --global --add safe.directory ${pwd()}"
        }
        
        dir('test-harness') {
            withEnv(["XKERNEL_ROOT=${pwd()}/..", "ARCH=${arch}"]) {
                sh "make ci-test run"
            }
        }
    }
}
